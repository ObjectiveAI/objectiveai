//! One signing key, ready to sign and to verify.

use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use jsonwebtoken::{DecodingKey, EncodingKey};
use openidconnect::core::{CoreJsonWebKey, CoreRsaPrivateSigningKey};
use openidconnect::{JsonWebKeyId, PrivateSigningKey as _};
use rsa::RsaPrivateKey;
use rsa::pkcs1::DecodeRsaPrivateKey as _;
use rsa::traits::PublicKeyParts as _;

use super::Error;

/// A signing key, in each form the issuer uses it in: the ID token's
/// signer, the access token's, the access token's verifier, and the
/// public key the key set publishes.
pub struct SigningKey {
    /// The key id.
    pub kid: String,
    /// The ID token's signer.
    pub(crate) id: CoreRsaPrivateSigningKey,
    /// The access token's signer.
    pub(crate) encoding: EncodingKey,
    /// The access token's verifier.
    pub(crate) decoding: DecodingKey,
    /// The public key, as the key set publishes it.
    pub(crate) public: CoreJsonWebKey,
}

impl SigningKey {
    /// The key `pem` is, a PKCS#1 RSA private key, as `kid`.
    pub fn from_pem(kid: &str, pem: &str) -> Result<SigningKey, Error> {
        let key = RsaPrivateKey::from_pkcs1_pem(pem).map_err(|error| Error::Pem(kid.to_string(), error.to_string()))?;
        let id = CoreRsaPrivateSigningKey::from_pem(pem, Some(JsonWebKeyId::new(kid.to_string())))
            .map_err(|error| Error::Pem(kid.to_string(), error))?;
        let encoding = EncodingKey::from_rsa_pem(pem.as_bytes()).map_err(Error::Jwt)?;
        let modulus = URL_SAFE_NO_PAD.encode(key.n().to_bytes_be());
        let exponent = URL_SAFE_NO_PAD.encode(key.e().to_bytes_be());
        let decoding = DecodingKey::from_rsa_components(&modulus, &exponent).map_err(Error::Jwt)?;
        let public = id.as_verification_key();
        Ok(SigningKey { kid: kid.to_string(), id, encoding, decoding, public })
    }
}
