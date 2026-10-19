//! `POST /secrets/collect`: the rotating secrets, handed back.
//!
//! The proxy sends the [`request::Request`] JSON — the names of every
//! `rotating` secret the program declared — at every inactive: an
//! agent's loop ended, a tool's last exchange in flight answered. A
//! `2xx` carries the [`response::Response`] JSON, EVERY name asked
//! for with the value the program leaves in it, which the caller
//! writes back into the place the value was locked from before it
//! drops the lock. A non-`2xx`, a body that is not the response, or a
//! name missing from it is the program failing to give a value back,
//! and ends the run with an error naming the secret and never a
//! value: a locked value whose fate is unknown is not written back,
//! and is released as it was. A program that declared no `rotating`
//! secret is never called.

pub mod request;
pub mod response;
