//! A minimal client of the OCI Distribution API: the read side, by
//! digest, against the registries the configuration lists — what a
//! push, a `has`, a pull and a hash fetch from a registry with, ONE
//! client for the provider, so a repository's token serves all four.
//!
//! # What it asks
//!
//! 1. A manifest: `GET https://<host>/v2/<name>/manifests/<digest>`,
//!    accepting the OCI and Docker manifest and index media types
//!    ([`ACCEPT`](crate::host::image_manager::manifest::ACCEPT)). `404`
//!    is not served; any other answer but `200` is the client's error.
//! 2. A blob: `GET https://<host>/v2/<name>/blobs/<digest>`, its body
//!    read piece by piece, redirects followed — a registry commonly
//!    answers a blob with a redirect to its storage — with the
//!    authorization dropped at a redirect to another host, which is
//!    reqwest's own rule.
//!
//! What either answers is verified by its reader, against the digest;
//! nothing here trusts a byte.
//!
//! # How it authenticates
//!
//! 1. Every request is first sent as the last one to the same
//!    repository was: with the Bearer token it got, or the Basic
//!    credential, or nothing.
//! 2. A `401` is read for its `WWW-Authenticate` challenge. `Basic` is
//!    answered with the registry's configured
//!    [`Credential`](crate::config::containers::Credential), when it has
//!    one. `Bearer` is answered by asking the challenge's `realm` for a
//!    token, with its `service` and `scope` as the query and the
//!    credential, when there is one, as Basic — anonymous otherwise —
//!    and the token, `token` or `access_token` of the answer, sent as
//!    `Bearer`. The request is sent once more; a second `401` is the
//!    client's error.
//! 3. A token is kept per repository for the client's life and asked
//!    for again at the next `401`.
//!
//! # Hosts
//!
//! `docker.io` is reached at `registry-1.docker.io`, and a one-segment
//! name there is under `library/`, as every client of Docker Hub
//! names it.
//!
//! [`RegistryClient`] is the client.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod registry_client;

pub use registry_client::*;
