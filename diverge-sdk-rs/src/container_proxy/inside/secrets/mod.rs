//! The two calls over a program's secrets: the values fed down, and
//! the rotating ones collected back.
//!
//! The program declares which secrets it takes, and how, in its
//! answer to [`register`](super::register); the caller resolves them,
//! and from then on the values travel two ways, each a call of the
//! proxy's on the program's own server. [`feed`] is `POST /secrets`:
//! the values, every name the program declared, handed to the program
//! once the caller has them — first when the registration is
//! answered, before the container's id goes out, and again at every
//! activation after; the program holds them until the call answers,
//! and nothing is delivered to it before. [`collect`] is
//! `POST /secrets/collect`: the proxy naming the `rotating` secrets
//! and the program answering each with the value it leaves in it,
//! at every inactive — an agent's loop ended, a tool's last exchange
//! in flight answered — which the caller writes back into the place
//! the value was locked from. A program that declared no secret is
//! never called on either.

pub mod collect;
pub mod feed;
