//! One volume a provider offers.

use serde::{Deserialize, Serialize};

/// A directory a caller may mount, under the name a provider gave it.
///
/// [`name`](Self::name) is what to call it, [`bytes`](Self::bytes) is
/// how big it is, and [`created`](Self::created) is how old it is.
/// That is the whole of what a listing says about one, and the
/// omissions are the interesting part.
///
/// # What is inside it is a stat away
///
/// How much of it is used and the hash of its content are what a
/// [`stat`](crate::endpoints::volumes::stat) reports, because each
/// costs a walk of the volume and a listing does not pay for one. A
/// listing is what a provider knows without looking.
///
/// # Where it is, is not here
///
/// A volume carries no path. Not a private one, not an opaque one —
/// none, because a caller has nothing to do with one.
///
/// Everything a caller does with a volume goes through its name: a
/// [`stat`](crate::endpoints::volumes::stat) names it, a
/// [`delete`](crate::endpoints::volumes::delete) names it, and a
/// [`VolumeMount`](crate::shared::containers::request::VolumeMount)
/// names it, and a provider looks the name up rather than resolving
/// anything. A path would be the one field nothing consumes, and a
/// field nothing consumes is one that gets consumed anyway — a caller
/// building strings out of it, a provider then unable to move a volume
/// without breaking someone.
///
/// The paths a mount's
/// [`host_relative_path`](crate::shared::containers::request::VolumeMount::host_relative_path)
/// names still mean what they meant. They are relative to the volume;
/// the volume is simply no longer described in terms of anywhere
/// else.
///
/// # Why a volume rather than a directory
///
/// Because the name is the point. A directory is a thing on a disk; a
/// volume is a thing a provider decided to OFFER, and the offering is
/// what a caller interacts with. It is also what a
/// [`VolumeMount`](crate::shared::containers::request::VolumeMount)
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
    /// [`stat`](crate::endpoints::volumes::stat) names a volume by
    /// this and by nothing else, so two volumes in one listing sharing
    /// a name would make one of them unreachable.
    pub name: String,
    /// How big it is, in BYTES.
    ///
    /// For a volume a
    /// [`create`](crate::endpoints::volumes::create::client::request::Frame::bytes)
    /// made, the number that was asked for.
    pub bytes: u64,
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
