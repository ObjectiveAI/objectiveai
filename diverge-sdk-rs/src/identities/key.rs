//! An identity's key: the private half of what its tokens are bound
//! to.

use std::io;

use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use p256::ecdsa::signature::Signer as _;
use p256::ecdsa::{Signature, SigningKey};
use rand::RngCore as _;
use serde::{Deserialize, Serialize};
use tokio::io::AsyncWriteExt as _;
use zeroize::{Zeroize as _, Zeroizing};

use super::Error;
use super::name::Paths;
use crate::wire::frame::auth::{POINT_LEN, SIGNATURE_LEN, THUMBPRINT_LEN};
use crate::wire::handshake::{Signer, thumbprint};

/// An identity's key on this host: a P-256 key pair, whose public
/// half every access token of the identity is bound to — by its
/// thumbprint, in the token's `cnf.jkt` — and whose private half signs
/// each proof sent to the issuer and each handshake the identity is
/// presented in. The rules:
///
/// 1. **One per identity, per host.** The key is kept beside the
///    identity's file, as `<subject>.key`, and never leaves the host.
///    The same identity signed in on another host has another key
///    there.
/// 2. **Made by itself, when first needed.** A login makes the key
///    where the identity has none, and so does the first token asked
///    for of an identity that was signed in before it had one. Nothing
///    else makes a key, and nobody is asked to.
/// 3. **Never replaced.** A key that exists is the identity's for as
///    long as the identity is on the host: the issuer binds the
///    identity's refresh token to it at the login, and no other key
///    spends that refresh token. An identity whose key file was taken
///    away signs in again.
/// 4. **A key of another kind is none.** A key file that holds an
///    Ed25519 key — what a key was before tokens were bound as RFC
///    9449 binds them — is read as no key, and replaced by a new one:
///    no issuer ever bound a refresh token to it.
/// 5. **Sent only as its public half.** The public half goes to the
///    issuer inside each proof, and to a peer in each handshake. The
///    private half is read by this module alone, to sign.
/// 6. **Kept as the file is.** Written whole to a staging file, made
///    owner-only, and renamed over the key, under the identity's lock.
///    The file is a JSON Web Key, RFC 7518 §6.2, private part
///    included.
/// 7. **Gone with the identity.** Removing an identity removes its
///    key.
///
/// It has no `Serialize` and no `Display`, its `Debug` prints
/// `Key(<hidden>)`, and its memory is zeroed when it is dropped.
#[derive(Clone)]
pub struct Key(SigningKey);

/// The key's file: a JSON Web Key with its private part. `y` is absent
/// from a key of the former kind, which is all this reads of one.
#[derive(Serialize, Deserialize)]
struct Stored {
    kty: String,
    crv: String,
    /// The `x` coordinate, as unpadded base64url.
    x: String,
    /// The `y` coordinate, as unpadded base64url.
    #[serde(default)]
    y: String,
    /// The private key, as unpadded base64url.
    d: String,
}

/// The private part is zeroed as the file's contents are let go.
impl Drop for Stored {
    fn drop(&mut self) {
        self.d.zeroize();
    }
}

/// The key type every key here is.
const KTY: &str = "EC";

/// The curve every key here is on.
const CRV: &str = "P-256";

/// The key type of a key of the former kind — rule 4.
const FORMER: &str = "OKP";

impl Key {
    /// A new key, from the system's randomness. A draw that is no
    /// scalar of the curve is drawn again.
    fn generate() -> Key {
        loop {
            let mut seed = Zeroizing::new([0u8; 32]);
            rand::rng().fill_bytes(seed.as_mut());
            if let Ok(key) = SigningKey::from_slice(seed.as_ref()) {
                return Key(key);
            }
        }
    }

    /// The public half: the uncompressed point, sixty-five bytes.
    pub fn public(&self) -> [u8; POINT_LEN] {
        let point = self.0.verifying_key().to_encoded_point(false);
        let mut public = [0u8; POINT_LEN];
        public.copy_from_slice(point.as_bytes());
        public
    }

    /// The two coordinates of the public half, each as unpadded
    /// base64url.
    fn coordinates(&self) -> (String, String) {
        let public = self.public();
        let (x, y) = public[1..].split_at(32);
        (URL_SAFE_NO_PAD.encode(x), URL_SAFE_NO_PAD.encode(y))
    }

    /// The public half as the JSON Web Key a proof carries:
    /// `{"kty":"EC","crv":"P-256","x":"…","y":"…"}`, those four
    /// members and no other.
    pub fn jwk(&self) -> String {
        let (x, y) = self.coordinates();
        format!(r#"{{"kty":"{KTY}","crv":"{CRV}","x":"{x}","y":"{y}"}}"#)
    }

    /// The public half's RFC 7638 thumbprint: what a token bound to
    /// this key carries as `cnf.jkt`.
    pub fn thumbprint(&self) -> [u8; THUMBPRINT_LEN] {
        thumbprint(&self.public())
    }
}

impl Signer for Key {
    fn sign(&self, message: &[u8]) -> [u8; SIGNATURE_LEN] {
        let signature: Signature = self.0.sign(message);
        signature.to_bytes().into()
    }
}

impl std::fmt::Debug for Key {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Key(<hidden>)")
    }
}

/// The key at `paths`, or `None` when there is no such file, or the
/// file holds a key of the former kind — rule 4. Any other file that
/// is not a key this module wrote is [`Error::Key`]. The caller holds
/// the lock.
pub(super) async fn read(paths: &Paths) -> Result<Option<Key>, Error> {
    let bytes = match tokio::fs::read(&paths.key).await {
        Ok(bytes) => Zeroizing::new(bytes),
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(source) => return Err(Error::Io { path: paths.key.clone(), source }),
    };
    let not_one = |why: &str| Error::Key { path: paths.key.clone(), why: why.to_string() };
    let stored: Stored = serde_json::from_slice(&bytes).map_err(|error| not_one(&error.to_string()))?;
    if stored.kty == FORMER {
        return Ok(None);
    }
    if stored.kty != KTY || stored.crv != CRV {
        return Err(not_one("it is not a P-256 key"));
    }
    let scalar = Zeroizing::new(URL_SAFE_NO_PAD.decode(stored.d.as_bytes()).map_err(|_| not_one("its private part is not base64url"))?);
    let key = Key(SigningKey::from_slice(&scalar).map_err(|_| not_one("its private part is no key of the curve"))?);
    // The public part is the private part's own: a file whose two
    // halves disagree was not written here.
    let (x, y) = key.coordinates();
    if x != stored.x || y != stored.y {
        return Err(not_one("its public part is not its private part's"));
    }
    Ok(Some(key))
}

/// Write `key` at `paths`, as rule 6 says. The caller holds the lock,
/// and the folder exists.
pub(super) async fn write(paths: &Paths, key: &Key) -> Result<(), Error> {
    let staging = &paths.key_staging;
    let failed = |source| Error::Io { path: staging.clone(), source };
    let (x, y) = key.coordinates();
    let stored = Stored { kty: KTY.to_string(), crv: CRV.to_string(), x, y, d: URL_SAFE_NO_PAD.encode(key.0.to_bytes()) };
    let bytes = Zeroizing::new(serde_json::to_vec_pretty(&stored).map_err(|source| Error::Json { path: staging.clone(), source })?);
    let mut options = tokio::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    options.mode(0o600);
    let mut file = options.open(staging).await.map_err(failed)?;
    file.write_all(&bytes).await.map_err(failed)?;
    file.sync_all().await.map_err(failed)?;
    drop(file);
    tokio::fs::rename(staging, &paths.key).await.map_err(|source| Error::Io { path: paths.key.clone(), source })
}

/// The key at `paths`, made and written where there is none — rules 2
/// and 4. The caller holds the lock, and the folder exists.
pub(super) async fn ensure(paths: &Paths) -> Result<Key, Error> {
    if let Some(key) = read(paths).await? {
        return Ok(key);
    }
    let key = Key::generate();
    write(paths, &key).await?;
    Ok(key)
}
