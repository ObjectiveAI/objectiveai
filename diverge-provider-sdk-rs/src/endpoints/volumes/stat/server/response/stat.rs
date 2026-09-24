//! One volume, examined.

use serde::{Deserialize, Serialize};

use crate::endpoints::volumes::list::server::response::Volume;

/// What a listing says about a volume, and the two things it does not.
///
/// [`volume`](Self::volume) is the listing's own record of it, the same
/// four fields a [`list`](crate::endpoints::volumes::list) reports;
/// [`bytes_used`](Self::bytes_used) and [`dirhash`](Self::dirhash) are
/// what a listing leaves out, because each costs a walk of the volume
/// and a stat pays for one volume's walk on purpose.
///
/// # Nested rather than flattened
///
/// The listing's [`Volume`] sits inside this as a field rather than
/// having its fields copied in, so there is one definition of what a
/// listing says. Postcard writes a nested struct as its fields in
/// place, so the wire reads `name`, `bytes`, `created`, `persist`,
/// then the two fields of its own — a listing's record with two
/// fields appended.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Stat {
    /// The volume as a listing reports it: its name, its size, when
    /// it came into being, and whether it keeps what is written into
    /// it.
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
    /// The string that Go's
    /// [`golang.org/x/mod/sumdb/dirhash`](https://pkg.go.dev/golang.org/x/mod/sumdb/dirhash)
    /// returns from `HashDir(root, "", Hash1)` for the volume's root,
    /// as that package defines it, which is this. The files are every
    /// entry beneath the root that is not a directory as `lstat`
    /// reports it, each named by its path relative to the root with
    /// its components joined by `/`; a symbolic link is a file, opened
    /// through the link. The names are sorted bytewise. A name
    /// containing a newline is an error. For each file, in that order,
    /// one line is written into one SHA-256: the SHA-256 of the file's
    /// bytes as lowercase hexadecimal, two spaces, the name, and a
    /// newline. The hash is the string `h1:` followed by that SHA-256
    /// encoded as standard base64 with padding.
    ///
    /// A volume with no file has the hash
    /// `h1:47DEQpj8HBSa+/TImW+5JCeuQeRkm5NMpJWZG3hSuFU=`, which is what
    /// a [`create`](crate::endpoints::volumes::create) just made.
    ///
    /// # Why a hash rather than a version
    ///
    /// Two stats with one `dirhash` have one content between them;
    /// two with different ones do not. That is the whole of what it
    /// says, and a counter could not say it across providers.
    pub dirhash: String,
}
