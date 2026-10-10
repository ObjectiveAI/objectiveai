//! The ids records are known by inside the daemon.

/// An account's row id. Never on the wire — accounts are named there
/// by name or identity — and a type of its own so that it cannot be
/// handed to a query about roles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct AccountId(pub i64);

/// A role's row id, likewise.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RoleId(pub i64);

/// An outgoing provider's row id, likewise.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct OutgoingId(pub i64);

/// An incoming credential's row id, likewise.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct IncomingId(pub i64);

/// A daemon record's row id, likewise.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DaemonId(pub i64);

/// An agent's row id. Never on the wire — an agent is named there by
/// its name, or by its template and index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct AgentId(pub i64);

/// A tool's row id, likewise.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ToolId(pub i64);
