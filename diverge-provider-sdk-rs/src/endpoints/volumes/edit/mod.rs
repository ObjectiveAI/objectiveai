//! Changing a volume's reserved size, its persist mode, or both.
//!
//! A caller names one and states the change. Split by who SENDS, as
//! everywhere else: the ask is in [`client`], the answer in [`server`].
//!
//! The size and the persist mode are the two things about a volume
//! that can be changed, and an edit changes one or both. Its name is
//! the handle and everything else is a consequence — what a
//! [`list`](super::list) reports beside them either follows from the
//! contents or was fixed when the volume came into being. Nothing is
//! changed while any container has the volume: an edit takes the
//! volume to itself, and is refused when it cannot.

pub mod client;
pub mod server;
