//! Whether a client has a secret.

/// A client's kind, as OAuth 2.0 §2.1 defines it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Kind {
    /// A client that cannot keep a secret — the CLI, the viewer, a
    /// browser's page — and presents its id alone. It proves the
    /// request is its own by PKCE, or by the device code.
    Public,
    /// A client that keeps a secret, and presents it with its id.
    Confidential,
}

impl Kind {
    /// The kind as the database writes it.
    pub fn name(self) -> &'static str {
        match self {
            Kind::Public => "public",
            Kind::Confidential => "confidential",
        }
    }

    /// The kind the database wrote as `name`.
    pub fn from_name(name: &str) -> Option<Kind> {
        match name {
            "public" => Some(Kind::Public),
            "confidential" => Some(Kind::Confidential),
            _ => None,
        }
    }
}
