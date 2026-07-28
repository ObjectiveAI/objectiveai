//! The viewer-plugin bundle download —
//! [`Client::get_viewer_plugin`](super::Client::get_viewer_plugin)'s
//! response. LOW-LEVEL by design: raw tar.gz bytes, caller-paced, no
//! filesystem — the consumer (the viewer's installer) un-tars into
//! its own staging its own way.

use futures::TryStreamExt;

/// One in-flight viewer-extension bundle: the tag's commit SHA (from
/// the `X-OBJECTIVEAI-SHA` response header) plus the tar.gz byte
/// stream. Dropping it aborts the transfer.
pub struct ViewerPlugin {
    /// The plugin tag's commit SHA, when the daemon stamped it.
    pub commit_sha: Option<String>,
    response: reqwest::Response,
}

impl ViewerPlugin {
    pub(crate) fn new(commit_sha: Option<String>, response: reqwest::Response) -> Self {
        Self {
            commit_sha,
            response,
        }
    }

    /// The next chunk of tar.gz bytes; `None` = the body ended. A
    /// TRUNCATED body is indistinguishable here from a complete one —
    /// the caller's un-gzip/un-tar is what validates completeness
    /// (a daemon-side failure mid-build never streams: the tar starts
    /// only after the build succeeded).
    pub async fn chunk(&mut self) -> Result<Option<bytes::Bytes>, super::Error> {
        Ok(self.response.chunk().await?)
    }

    /// The remaining body as a byte stream.
    pub fn bytes_stream(
        self,
    ) -> impl futures::Stream<Item = Result<bytes::Bytes, super::Error>> {
        self.response.bytes_stream().map_err(super::Error::from)
    }
}
