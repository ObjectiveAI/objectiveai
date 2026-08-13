//! The head of a registry answer.

/// The status and headers of a registry answer.
///
/// Where `Content-Length`, `Content-Type` and `Docker-Content-Digest`
/// arrive — a runtime cannot verify what it did not know the size or
/// the digest of, so this is not decoration. `Content-Range` rides
/// here too, on the `206` that answers a resumed pull.
///
/// See [`http::response::Head`](crate::http::response::Head).
pub type Head = crate::http::response::Head;
