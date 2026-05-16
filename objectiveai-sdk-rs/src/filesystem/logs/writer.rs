use std::collections::HashMap;
use std::path::PathBuf;

use super::LogFile;

/// Writes streaming chunks to the log file structure on disk.
///
/// `C` is the chunk type. The `produce` function pointer extracts
/// [`LogFile`]s from each chunk.
///
/// Maintains a buffer of previously written file contents so that
/// unchanged files are not rewritten on every chunk.
pub struct LogWriter<C> {
    logs_dir: PathBuf,
    produce: fn(&C) -> Option<Vec<LogFile>>,
    primary_id: Option<String>,
    buffer: HashMap<String, Vec<u8>>,
}

impl<C> LogWriter<C> {
    pub fn new(
        logs_dir: PathBuf,
        produce: fn(&C) -> Option<Vec<LogFile>>,
    ) -> Self {
        Self {
            logs_dir,
            produce,
            primary_id: None,
            buffer: HashMap::new(),
        }
    }

    /// The ID of the primary (root) log entry.
    ///
    /// Returns `None` until at least one chunk has been written.
    pub fn primary_id(&self) -> Option<&str> {
        self.primary_id.as_deref()
    }

    /// Write a chunk to disk. Files whose content hasn't changed since the
    /// last write are skipped.
    pub async fn write(&mut self, chunk: &C) -> Result<(), super::super::Error> {
        let files = match (self.produce)(chunk) {
            Some(files) => files,
            None => return Ok(()),
        };

        // The last file is always the root — capture its id on first write
        if self.primary_id.is_none() {
            if let Some(last) = files.last() {
                self.primary_id = Some(last.id.clone());
            }
        }

        // Filter out files whose content matches the buffer
        let changed: Vec<LogFile> = files.into_iter().filter(|file| {
            let path = file.path();
            if self.buffer.get(&path).map_or(false, |prev| prev == &file.content) {
                return false;
            }
            self.buffer.insert(path, file.content.clone());
            true
        }).collect();

        futures::future::try_join_all(changed.into_iter().map(|file| {
            let full_path = self.logs_dir.join(file.path());
            async move {
                if let Some(parent) = full_path.parent() {
                    tokio::fs::create_dir_all(parent).await
                        .map_err(|e| super::super::Error::Write(parent.to_path_buf(), e))?;
                }
                tokio::fs::write(&full_path, file.content).await
                    .map_err(|e| super::super::Error::Write(full_path, e))
            }
        })).await?;

        Ok(())
    }
}
