//! The directory a build reads from, and whose it is.

/// A build's [`Context`](diverge_sdk::provider::endpoints::images::build::client::request::Context)
/// with the one thing a caller could not say: whose volume
/// [`volume_name`](Self::volume_name) names. The same two fields, in
/// the same order, behind a third that came from somewhere else —
/// the identity a provider attached on arrival, for the reason
/// [`Mount`](super::mount::Mount) gives: a volume's name is unique
/// within the caller it was listed to, and a bare name resolves to
/// nothing on its own.
///
/// # What a manager does with it
///
/// Resolves it to a directory of its own volume, by whatever layout
/// it keeps volumes in, and builds with that directory as the
/// build's context and working directory, reading it and writing
/// nothing into it. The handler has taken the volume's shared hold
/// before the build and gives it back after, so the directory is as
/// a mount would see it for the build's life; the manager takes no
/// hold of its own.
///
/// # It is opaque here
///
/// This crate never mints an identity, never parses one, and never
/// compares two; see [`Mount`](super::mount::Mount).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct Context {
    /// Whose volume [`volume_name`](Self::volume_name) is.
    pub client_identity: String,
    /// Which volume, by the name a listing gave it.
    pub volume_name: String,
    /// How far into that volume the context is, as path components
    /// relative to it. Empty is the volume itself. A provider descends
    /// names and never resolves a path; the handler has refused a
    /// component that is not a name before this is made.
    pub volume_relative_path: Vec<String>,
}
