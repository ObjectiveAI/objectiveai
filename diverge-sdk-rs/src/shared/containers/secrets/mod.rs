//! The secrets a container is handed: named by its template, moded by
//! its program, valued by the caller.
//!
//! A template declares secrets by NAME alone, and the record made
//! from it binds each name to a vault key of the caller's. Neither
//! says what the program does with a secret: that is the program's
//! own answer at its registration, a [`Declaration`] — each name it
//! takes, to the [`Mode`] it takes it in. A `fixed` secret is read
//! once per activation, a random value among the key's, and never
//! locked; a `rotating` secret is one value of the key's, locked to
//! the container for as long as it is active and written back when it
//! goes inactive, so that a value the program changed — a refreshed
//! token, a rolled credential — is the value the next activation
//! finds. The caller feeds the [`Values`] — each name to its value,
//! any JSON — when it answers the registration and at every
//! activation after, and collects the rotating ones at every
//! inactive.
//!
//! Three shapes, one home: the [`Declaration`] rides the registration
//! ask — see [`registration`](super::registration) — and the
//! [`Values`] ride its answer, every `secrets` channel a caller opens
//! on a run and a provider on a begin, and every `Inactive` on the
//! main streams. [`feed`] is what a `secrets` channel answers: fed, or
//! why not.

mod declaration;
mod mode;
mod values;

pub mod feed;

pub use declaration::*;
pub use mode::*;
pub use values::*;
