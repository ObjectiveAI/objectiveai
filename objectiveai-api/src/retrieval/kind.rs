//! Resource kind for retrieval operations.

/// The kind of resource being retrieved.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Kind {
    Agents,
    Swarms,
    Functions,
    Profiles,
}

impl Kind {
    /// Returns the directory name for this kind.
    pub fn as_str(self) -> &'static str {
        match self {
            Kind::Agents => "agents",
            Kind::Swarms => "swarms",
            Kind::Functions => "functions",
            Kind::Profiles => "profiles",
        }
    }
}
