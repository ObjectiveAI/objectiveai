//! How large a volume may be made.
//!
//! One question with no parameters, and one number: the largest size,
//! in bytes, the provider can reserve for a new volume of this
//! caller, as of now. Split by who SENDS, as everywhere else: the
//! question is in [`client`], the answer in [`server`].
//!
//! # The largest volume, not the sum of the room
//!
//! A provider may have room in several places, and a volume lives in
//! one of them. So the number is the largest single volume that can
//! be made, which is the bound a [`create`](super::create) is actually
//! held to — not the total, which no single volume could ever use.
//!
//! # It reserves nothing
//!
//! The number is a fact about the instant it was answered. A
//! [`create`](super::create) sized by it may still be answered
//! insufficient capacity if the room went elsewhere in between, and
//! that answer is the one that counts.

pub mod client;
pub mod server;
