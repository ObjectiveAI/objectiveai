//! Providers: the two ways the daemon comes to be connected to one.
//!
//! A provider is where containers run and volumes live, and the daemon
//! reaches it one of two ways, which its
//! [`Identity`](crate::daemon::endpoints::agents::logs::server::response::Identity)
//! already tells apart. An [`outgoing`] provider the daemon DIALS: a
//! client gives an address and a mode — unbrokered, with the credential
//! the daemon presents as its own — and the daemon dials it for
//! whatever it needs of it, presenting that credential first, as the
//! wire rules. An [`incoming`] provider dials the daemon, and the
//! daemon JUDGES its credential as a provider judges its own peers: the
//! client gives judges — a key that the credential must equal and the
//! identity it then names, or a hook that judges the credential itself
//! and names the identity — and the daemon tries them in the order they
//! were added; the first that accepts decides, and none accepting is a
//! closed connection.
//!
//! # A hook is a resource
//!
//! The hook judge of an incoming provider is a directory
//! [resource](crate::daemon::endpoints::resources) the caller holds
//! already, with `hook.yaml` at its root: the manifest the provider
//! server reads, a command per platform — `windows`, `macos`, `linux`,
//! each a whole argv — of which the daemon's platform's is run. The
//! daemon writes the hook one line of JSON on stdin,
//! `{"credential":…,"address":…}`, the credential as the peer presented
//! it and the peer's address as the OS reported it, and reads one JSON
//! document from stdout on exit `0`: `{"authorized":true,"identity":…}`
//! accepts the peer as that identity, `{"authorized":false}` refuses
//! it, and anything else — a non-zero exit, any other output — is a
//! refusal.
//!
//! # Secrets are given, never answered
//!
//! An outgoing provider's authorization and a key judge's key are
//! credentials. A list and a get report the mode or the judge without
//! them; an edit replaces them whole, which is how one rotates.

pub mod incoming;
pub mod outgoing;
