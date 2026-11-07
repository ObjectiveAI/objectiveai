//! Running one command in a container, its output streamed.
//!
//! The command is an argv run directly — its first item a path, or a
//! name looked up on the container's `PATH` — with no shell between:
//! the one form every image can run, a distroless or scratch one
//! among them. A caller that wants a shell names it, `["sh", "-c",
//! "…"]`. The command reads nothing: its stdin is the null device.
//!
//! What comes back is a stream ([`response::Frame`]):
//!
//! 1. Stdout and stderr, each a frame per read of at most
//!    [`CHUNK_SIZE`](crate::CHUNK_SIZE), in the order the reads
//!    completed; each pipe's bytes in their order.
//! 2. Exactly one `Exited` or `Signaled`, last, once both pipes are at
//!    their end; the finish follows it.
//! 3. A command that never ran — not found, not executable, a working
//!    directory that is not one — is exactly one `Error`, and the
//!    finish.
//! 4. The reader paces it, as every stream is paced (see
//!    [`credit`](crate::wire::credit)): a reader that falls behind
//!    fills the pipe, and the command waits on its write.
//! 5. A cancel, the reader abandoning the stream, or the connection
//!    ending kills the command's whole process group: it answers
//!    nothing more.

pub mod request;
pub mod response;
