//! One channel, where routing stops.
//!
//! The end of the tree: a channel has nothing under it, so this
//! delivers payloads to one consumer rather than choosing between
//! several. It is named for its place in the tree rather than for
//! doing any routing of its own.
//!
//! # Empty
//!
//! Nothing here yet. One thing about its shape is settled.
//!
//! **Both directions are the same type.** A channel this end opened
//! and a channel the far end opened differ only in who sends the
//! request and who sends the finish — the numbering spaces are already
//! separate, so nothing about the delivery differs. What differs is
//! which end may end it: only a responder finishes a channel, so a
//! channel this end opened is one this end cannot close.
