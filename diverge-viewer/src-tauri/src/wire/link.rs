//! The link, as Rust holds it between uses.

use diverge_sdk::wire::client::handle::{Handle, SendError};
use diverge_viewer_config::Daemon;
use tokio::sync::Mutex;

use super::{WireStatus, dial};

/// The link as it stands: not yet dialled, up, or down with the reason.
enum Link {
    Idle,
    Up(Handle),
    Down(String),
}

/// The daemon the viewer dials, and the one connection to it: dialled
/// on first use, kept while it lives, taken down when a send finds it
/// gone or the page asks, and dialled again on the use after. Every
/// command holds this through Tauri's state.
pub struct Wire {
    daemon: Daemon,
    link: Mutex<Link>,
}

impl Wire {
    /// A link to `daemon`, not yet dialled.
    pub fn new(daemon: Daemon) -> Wire {
        Wire { daemon, link: Mutex::new(Link::Idle) }
    }

    /// The handle, dialling when there is none up: a link not yet
    /// dialled, or down, is dialled now, so a failure is retried by
    /// the next use. The lock is held through the dial, so uses that
    /// race the first wait for its one connection rather than making
    /// their own.
    pub async fn handle(&self) -> Result<Handle, String> {
        let mut link = self.link.lock().await;
        if let Link::Up(handle) = &*link {
            return Ok(handle.clone());
        }
        match dial(&self.daemon.address, &self.daemon.key).await {
            Ok(handle) => {
                *link = Link::Up(handle.clone());
                Ok(handle)
            }
            Err(error) => {
                let message = error.to_string();
                *link = Link::Down(message.clone());
                Err(message)
            }
        }
    }

    /// What a send came back with, as a command answers it. A send
    /// that found the connection or its router gone takes the link
    /// down, so the next use dials again; a scope that is not open
    /// says nothing about the connection and leaves it as it is.
    pub async fn sent<T>(&self, result: Result<T, SendError>) -> Result<T, String> {
        match result {
            Ok(value) => Ok(value),
            Err(error) => {
                let message = error.to_string();
                if !matches!(error, SendError::Scope) {
                    *self.link.lock().await = Link::Down(message.clone());
                }
                Err(message)
            }
        }
    }

    /// How the link stands, without dialling: `dialling` while a use
    /// holds the lock, which only a dial does for long.
    pub fn status(&self) -> WireStatus {
        match self.link.try_lock() {
            Err(_) => WireStatus::Dialling,
            Ok(link) => match &*link {
                Link::Idle => WireStatus::Idle,
                Link::Up(_) => WireStatus::Connected,
                Link::Down(message) => WireStatus::Failed { message: message.clone() },
            },
        }
    }

    /// Drop the link, whatever it was: the next use dials again. The
    /// page's remedy for a connection that is up in name and answers
    /// nothing — a daemon that went away without a send noticing.
    pub async fn reset(&self) {
        *self.link.lock().await = Link::Idle;
    }
}
