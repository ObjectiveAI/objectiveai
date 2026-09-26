//! Volume serve: the FUSE asks, answered from one of the daemon's
//! volumes.
//!
//! A client names a volume of the daemon's and holds the scope; for
//! as long as it does, the daemon holds the volume mounted and
//! answers, on the channels the client opens, the nine asks a FUSE
//! mount makes — a stat, a piece read, a piece written, a truncate, a
//! setattr, a listing, a removal, a rename, a mkdir — from the
//! volume, a piece at a time. The client's stop ends it.
//!
//! The exchange is the provider's [`diverge_provider_sdk::endpoints::volumes::serve`] — the same request
//! after the tag, the same asks, the same answers, the same stop —
//! and its rule in full is stated there; only the tag and the host
//! differ. Split by who SENDS, as everywhere else: the ask and the
//! stop are in [`client`], the answers in [`server`].

pub mod client;
pub mod server;
