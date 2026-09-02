//! Which files a delivery put on disk.

use super::{MEMORY_TAG, STATE_DB_TAG, USER_TAG};

/// Which of the three files a delivery put on disk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Landed {
    /// `state.db` — every real continuation has one; `false` here is
    /// the fresh start.
    pub state_db: bool,
    /// `memories/MEMORY.md`.
    pub memory: bool,
    /// `memories/USER.md`.
    pub user: bool,
}

impl Landed {
    /// Nothing landed: the fresh start.
    pub const NONE: Landed = Landed {
        state_db: false,
        memory: false,
        user: false,
    };

    /// Note that a tag's file now exists.
    pub(super) fn mark(&mut self, tag: u8) {
        match tag {
            STATE_DB_TAG => self.state_db = true,
            MEMORY_TAG => self.memory = true,
            USER_TAG => self.user = true,
            _ => {}
        }
    }
}
