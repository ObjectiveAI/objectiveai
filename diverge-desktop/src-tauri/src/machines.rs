//! The second seam: each machine's own volumes.
//!
//! Ronald took volumes off the daemon on 2026-09-25 (`87015ef92`, "the
//! daemon's volumes are gone"). A volume is a provider's own: named in
//! that provider's listing, kept on its disk, meaning nothing to any
//! other. So the ten verbs here are the PROVIDER protocol's
//! (`diverge_sdk::provider::endpoints::volumes`), addressed to one
//! machine, taking and returning that module's own types.
//!
//! An agent reaches a machine's volumes two ways, both stated on the
//! daemon's `agents::create` and `agents::edit`: pinned to the machine,
//! with its volumes mounted directly; or any machine's volume served live
//! across the daemon over FUSE.
//!
//! The wire doesn't say yet how the app reaches a machine's verbs. The SDK's
//! caller half (`volumes::*::client::execute`) can dial a machine the daemon
//! dials, not one that dials the daemon. Until it does, the stand-in daemon
//! answers for every machine it knows.
//! Nothing above this module knows which it holds.

use async_trait::async_trait;

use diverge_sdk::daemon::endpoints::agents::logs::server::response::Identity;
use diverge_sdk::provider::endpoints::volumes;
use diverge_sdk::shared::error::Error as WireError;

use crate::daemon::Frames;

/// `volumes::read`'s answer, owned. The SDK's frame borrows the bytes of
/// the wire frame it was read from; a stream cannot hand that out.
#[derive(Debug, Clone)]
pub enum ReadFrame {
    Body(Vec<u8>),
    Error(WireError),
}

impl ReadFrame {
    /// Exhaustive on purpose: a new answer from a provider breaks this.
    #[allow(dead_code)] // WireMachines'; the stand-in builds ReadFrames itself.
    pub fn from_wire(frame: volumes::read::server::response::Frame<'_>) -> Self {
        use volumes::read::server::response::Frame;
        match frame {
            Frame::Body(body) => ReadFrame::Body(body.0.to_vec()),
            Frame::Error(error) => ReadFrame::Error(error),
        }
    }
}

/// The provider protocol's volume verbs, tags 3–13 but `serve` (8), which
/// is the daemon's to hold for an agent's FUSE mounts, never the app's.
#[async_trait]
pub trait Machines: Send + Sync + 'static {
    /// Tag 3. What the machine offers.
    async fn volumes_list(&self, on: &Identity) -> volumes::list::server::response::Frame;

    /// Tag 4. One volume, examined: its size, its mode, what it uses.
    async fn volumes_stat(&self, on: &Identity, request: volumes::stat::client::request::Frame) -> volumes::stat::server::response::Frame;

    /// Tag 5. One file out of a volume, in pieces.
    fn volumes_read(&self, on: &Identity, request: volumes::read::client::request::Frame) -> Frames<ReadFrame>;

    /// Tag 6. One file into a volume, put in place whole. It lands in
    /// every mode: a mode governs what a container or a serve may change.
    async fn volumes_write(&self, on: &Identity, request: volumes::write::client::request::Frame, body: Vec<u8>) -> volumes::write::server::response::Frame;

    /// Tag 7. What a volume holds: one snapshot, not a watch.
    async fn volumes_filetree(&self, on: &Identity, request: volumes::filetree::client::request::Frame) -> volumes::filetree::server::response::Frame;

    /// Tag 9. How large a new volume may be.
    async fn volumes_create_capacity(&self, on: &Identity) -> volumes::create_capacity::server::response::Frame;

    /// Tag 10. Make a volume, in one of the three modes.
    async fn volumes_create(&self, on: &Identity, request: volumes::create::client::request::Frame) -> volumes::create::server::response::Frame;

    /// Tag 11. How far one may grow.
    async fn volumes_edit_capacity(&self, on: &Identity, request: volumes::edit_capacity::client::request::Frame) -> volumes::edit_capacity::server::response::Frame;

    /// Tag 12. Change its size, its mode, or both.
    async fn volumes_edit(&self, on: &Identity, request: volumes::edit::client::request::Frame) -> volumes::edit::server::response::Frame;

    /// Tag 13. Destroy one. Refused while anything holds it.
    async fn volumes_delete(&self, on: &Identity, request: volumes::delete::client::request::Frame) -> volumes::delete::server::response::Frame;
}
