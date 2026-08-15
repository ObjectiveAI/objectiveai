//! One volume a provider offers.

use serde::{Deserialize, Serialize};

/// A directory a caller may watch, under the name a provider gave it.
///
/// [`name`](Self::name) is what to call it, [`bytes`](Self::bytes) and
/// [`bytes_used`](Self::bytes_used) are how big it is and how much of
/// that is gone, and [`created`](Self::created) is how old it is. That
/// is the whole of what a listing says about one, and the omission is
/// the interesting part.
///
/// # Where it is, is not here
///
/// A volume carries no path. Not a private one, not an opaque one —
/// none, because a caller has nothing to do with one.
///
/// Everything a caller does with a volume goes through its name: a
/// [`watch`](crate::endpoints::volumes::watch) names it, a
/// [`delete`](crate::endpoints::volumes::delete) names it, and a
/// [`Mount`](crate::endpoints::laboratories::create::client::request::Mount)
/// names it, and a provider looks the name up rather than resolving
/// anything. A path would be the one field nothing consumes, and a
/// field nothing consumes is one that gets consumed anyway — a caller
/// building strings out of it, a provider then unable to move a volume
/// without breaking someone.
///
/// The paths inside a watch still mean what they meant. They are
/// relative to the volume; the volume is simply no longer described in
/// terms of anywhere else.
///
/// # Why a volume rather than a directory
///
/// Because the name is the point. A directory is a thing on a disk; a
/// volume is a thing a provider decided to OFFER, and the offering is
/// what a caller interacts with. It is also what a
/// [`Mount`](crate::endpoints::laboratories::create::client::request::Mount)
/// names, which is where the word already meant this.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Volume {
    /// What to call this volume, and how to ask for it.
    ///
    /// A label, and nothing derives it from anything: for a volume the
    /// provider offers, the provider chose it; for one a
    /// [`create`](crate::endpoints::volumes::create) made, the caller
    /// did. Nothing here says which, and nothing should.
    ///
    /// It is also the HANDLE, and the only one. A
    /// [`watch`](crate::endpoints::volumes::watch) names a volume by
    /// this and by nothing else, so two volumes in one listing sharing
    /// a name would make one of them unreachable.
    pub name: String,
    /// How big it is, in BYTES.
    ///
    /// For a volume a
    /// [`create`](crate::endpoints::volumes::create::client::request::Frame::bytes)
    /// made, the number that was asked for. For one the provider
    /// offers on its own, whatever bound the provider says applies —
    /// a quota, a device's capacity, a figure it decided on. Nothing
    /// distinguishes the two, and a caller that needs to has asked a
    /// question this listing does not answer.
    ///
    /// It does not change. A volume is the size it was made, and a
    /// caller that wants a bigger one makes a bigger one.
    pub bytes: u64,
    /// How much of it is gone, in BYTES.
    ///
    /// # A snapshot, and stale immediately
    ///
    /// Unlike every other field here, this one moves. It is true when
    /// the provider measured it and possibly not by the time it
    /// arrives — anything writing inside the volume changes it, and
    /// this protocol does not tell a caller when that happens.
    ///
    /// So it is worth reading as a rough gauge and not as a budget. A
    /// caller deciding whether a write will fit is racing every other
    /// writer, and losing that race looks like `ENOSPC` rather than
    /// like anything this listing said.
    ///
    /// # It is space consumed, not bytes written
    ///
    /// Which is not the same number. A sparse file counts what it
    /// occupies rather than how long it is, so writing a terabyte of
    /// zeroes may cost almost nothing; a filesystem's own metadata
    /// costs something for files that are empty. A caller summing the
    /// sizes in a [`filetree`](crate::shared::filetree) will not
    /// arrive at this figure and should not try.
    ///
    /// # It may exceed [`bytes`](Self::bytes)
    ///
    /// Rarely, and a reader should survive it rather than treat it as
    /// impossible. A quota with a grace allowance lets a writer past
    /// the limit before stopping it, and a provider that lowered a
    /// bound over an already-full volume reports what is true rather
    /// than what is tidy.
    pub bytes_used: u64,
    /// When the volume came into being, in SECONDS since the Unix
    /// epoch.
    ///
    /// What "came into being" means is the provider's to decide and
    /// the provider's alone — the directory's own creation time where
    /// that is knowable, when it was first offered where it is not.
    /// Nothing here can distinguish the two, and a caller that needs
    /// to has asked a question this listing does not answer.
    ///
    /// # Seconds, and an integer
    ///
    /// Seconds because nothing sorts volumes at finer resolution than
    /// that, and a field's precision is a promise about what varies
    /// between two values rather than about how many digits fit.
    ///
    /// An integer rather than a formatted timestamp because this rides
    /// a binary format. A date written as text inside postcard would
    /// be a text format smuggled into a binary one, carrying a
    /// timezone offset that is always the same, at a width that varies
    /// with the number it holds.
    ///
    /// Unsigned, so a volume cannot predate 1970. Nothing a provider
    /// offers does, and the alternative is a signed field whose
    /// negative half exists to represent a state that never occurs.
    pub created: u64,
}
