//! The replies lock.

use tokio::sync::{Mutex, mpsc};

use crate::response;

/// Where the reader's forwarded cancel replies arrive, or `None`
/// outside a run. Set by [`spawn`](super::spawn()) — BEFORE the
/// writer, so a writer seen always implies the replies are here —
/// and cleared with it.
///
/// Its own lock, apart from the writer's: reading the replies is a
/// right only this lock's holder has, which is what lets a dequeue
/// treat the replies it reads as answers to the cancels it wrote.
/// Only the dequeue ever takes BOTH locks at once — joined, not
/// nested — and everyone else touches one at a time, which is why no
/// lock-order cycle exists.
pub static REPLIES: Mutex<
    Option<mpsc::UnboundedReceiver<response::control::ControlResponse>>,
> = Mutex::const_new(None);
