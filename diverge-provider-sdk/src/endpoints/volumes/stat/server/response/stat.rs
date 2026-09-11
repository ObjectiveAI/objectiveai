//! One volume, examined.

use serde::{Deserialize, Serialize};

use crate::endpoints::volumes::list::server::response::Volume;

/// What a listing says about a volume, and the two things it does not.
///
/// [`volume`](Self::volume) is the listing's own record of it, the same
/// three fields a [`list`](crate::endpoints::volumes::list) reports;
/// [`bytes_used`](Self::bytes_used) and [`dirhash`](Self::dirhash) are
/// what a listing leaves out, because each costs a walk of the volume
/// and a stat pays for one volume's walk on purpose.
///
/// # Nested rather than flattened
///
/// The listing's [`Volume`] sits inside this as a field rather than
/// having its fields copied in, so there is one definition of what a
/// listing says. Postcard writes a nested struct as its fields in
/// place, so the wire reads `name`, `bytes`, `created`, then the two
/// fields of its own — a listing's record with two fields appended.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Stat {
    /// The volume as a listing reports it: its name, its size, and
    /// when it came into being.
    pub volume: Volume,
    /// How much of it is in use, in BYTES.
    ///
    /// The one field of a volume that changes on its own: writing
    /// inside the volume moves it, and nothing in this protocol
    /// reports when. It is what a stat was asked for, as of the
    /// moment the provider walked the volume.
    pub bytes_used: u64,
    /// The hash of the volume's content, at the time of the stat.
    ///
    /// The base64url SHA-256, unpadded, of the volume's manifest: one
    /// `<hash> <size> <path>` line per file, `<hash>` the base64url
    /// SHA-256 of the file's bytes, `<size>` its length in bytes,
    /// `<path>` relative to the volume's root and `/`-separated, the
    /// lines sorted bytewise. It is the hash half of the directory
    /// identity an
    /// [`IdentityMount`](crate::shared::containers::request::IdentityMount)
    /// carries, without the size — the listing already reports size.
    ///
    /// A volume with no file has the hash of the empty manifest,
    /// `47DEQpj8HBSa-_TImW-5JCeuQeRkm5NMpJWZG3hSuFU`, which is what a
    /// [`create`](crate::endpoints::volumes::create) just made.
    ///
    /// # Why a hash rather than a version
    ///
    /// Two stats with one `dirhash` have one content between them;
    /// two with different ones do not. That is the whole of what it
    /// says, and a counter could not say it across providers.
    pub dirhash: String,
}
