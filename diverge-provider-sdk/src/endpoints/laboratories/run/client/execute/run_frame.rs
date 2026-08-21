//! One thing a laboratory's scope said.

use super::super::super::server::response;
use crate::shared::filetree;

/// What a running laboratory reports.
///
/// What an [`ExecuteStream`](super::ExecuteStream) yields. The same
/// things
/// [`response::Frame`](super::super::super::server::response::Frame) carries,
/// minus the one that ends the scope: a failure is not an item here, it
/// is [`ExecuteStreamError::Provider`](super::ExecuteStreamError) and
/// the stream stops.
///
/// Which is the same relationship
/// [`McpFrame`](super::McpFrame) has to its wire frame, for a different
/// reason — that one drops nothing and owns its body, this one owns
/// everything already and drops a variant.
///
/// # Three kinds, and only one pair is ordered
///
/// A [`Disconnected`](Self::Disconnected) cannot arrive for a connector
/// that was not authorized first, and that is the only ordering in this
/// stream that means anything. Everything else interleaves as it
/// happens.
///
/// In particular the [`Id`](Self::Id) is not first. Its own
/// documentation says so — it "arrives whenever the provider has it,
/// which is not necessarily before the filesystem starts reporting" —
/// so a reader that holds tree changes until it has an id is holding
/// them for something that may come second.
#[derive(Debug, Clone, PartialEq)]
pub enum RunFrame {
    /// The container's id.
    ///
    /// What a prospective connector needs in order to
    /// [`connect`](crate::endpoints::laboratories::connect), and the
    /// only way it can be obtained — it reaches one out of band,
    /// because nothing in this protocol delivers it.
    ///
    /// Arrives once, whenever the provider has it.
    Id(response::Id),
    /// One change on the container's filesystem.
    ///
    /// A snapshot and then one frame per change, over the container's
    /// own root. What the sequence means, and what a reader has to hold
    /// to make sense of it, is [`filetree`]'s to say.
    ///
    /// It is the same stream a
    /// [`connector`](crate::endpoints::laboratories::connect) sees over
    /// the same tree, which is why running and watching are not two
    /// asks: a container's filesystem is the observable part of it
    /// running.
    Filetree(filetree::response::Frame),
    /// A connector that was attached is not any more.
    ///
    /// Carries the nickname the runner gave it when it authorized it,
    /// which is the entire reason a nickname exists — "one of them
    /// left" is not an answer to "which".
    ///
    /// # An event, not a count
    ///
    /// It says one connector left, not how many remain. A runner that
    /// wants a number keeps one: it answered every authorization, so it
    /// saw every arrival, and this is every departure.
    Disconnected(response::Disconnected),
}
