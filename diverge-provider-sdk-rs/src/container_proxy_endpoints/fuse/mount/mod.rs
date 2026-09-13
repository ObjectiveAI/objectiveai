//! Making one FUSE mount, and serving it.
//!
//! Split by who SENDS: [`client`] is the server's traffic, [`server`]
//! the proxy's.
//!
//! The scope a mount opens is the MOUNT's life, which is the proxy's:
//! nothing unmounts one. The server names a path and a kind; the
//! proxy makes the mount and answers `Ok` on channel `0` once it is
//! made, and then nothing more there for as long as the proxy lives —
//! or an error, and the finish. Every ask the mount makes from then
//! on — a stat, a read, a write, a listing, a removal, a rename, a
//! mkdir — is a channel the proxy opens on this scope. Which is why no
//! ask names the mount: the scope is the mount, and the id the caller
//! gave it stays with the server, which knows this scope by it.
//!
//! # Channels go one way here
//!
//! The proxy opens them; the server answers them. The server has
//! nothing to ask a mount, and opens no channel on this scope.

pub mod client;
pub mod server;
