//! One volume made visible inside a container, and whose it is.

/// A [`request::VolumeMount`] with the one thing a caller could not say.
///
/// The same three fields, in the same order, behind a fourth that came
/// from somewhere else:
/// [`client_identity`](Self::client_identity) is whose volume
/// [`host_name`](Self::host_name) names, and a provider attached it on
/// arrival rather than reading it off the wire.
///
/// # Why the wire type is not enough
///
/// Because [`host_name`](Self::host_name) is a
/// [`Volume::name`](crate::endpoints::volumes::list::server::response::Volume::name),
/// and a name is unique within the caller it was listed to — not
/// globally. Two callers each holding a volume they called `work` is
/// the ordinary case, not a collision to be prevented, because neither
/// was ever told about the other's.
///
/// So a bare name resolves to nothing on its own. What resolves it is
/// the pair, and this is the type that holds the pair.
///
/// # Why a caller does not send it
///
/// A caller cannot say whose volume it means, because the only answer
/// it could give is "mine". It asks on a connection, the connection is
/// what a provider authenticated, and every volume it can name is one
/// that connection was listed.
///
/// Which makes the field not merely redundant on the wire but
/// dangerous there: a caller that could state an owner could state
/// somebody else's, and a provider would be checking a claim instead
/// of knowing a fact. The identity is the provider's precisely because
/// the provider is the party that established it.
///
/// # Why it is on the mount rather than the deployment
///
/// Because the name it qualifies is on the mount. A
/// [`Deployment`](super::deployment::Deployment) that carried one
/// owner for all of them would put the two halves of a single fact in
/// two places, and would rule out a container drawing on more than one
/// caller's volumes — which nothing in this protocol has any reason to
/// rule out, and which a provider composing a deployment out of more
/// than one request would need.
///
/// Repeating an identity across several mounts costs a string per
/// mount and keeps every mount readable on its own.
///
/// # It is opaque here
///
/// This crate never mints one, never parses one, and never compares
/// two. It says only that a provider has some notion of which caller
/// it is talking to and writes that notion down here — an account, a
/// key id, a tenant, whatever authenticated the connection.
///
/// Nothing on the wire carries it, so nothing on the wire constrains
/// what it looks like.
///
/// [`request::VolumeMount`]: crate::shared::containers::request::VolumeMount
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct Mount {
    /// Whose volume [`host_name`](Self::host_name) is.
    ///
    /// The caller a provider resolves the name against. See the type's
    /// own documentation for why it is here and not on the wire.
    pub client_identity: String,
    /// Which offered volume, by the name a listing gave it.
    ///
    /// Names come from
    /// [`Volume::name`](crate::endpoints::volumes::list::server::response::Volume::name)
    /// and mean nothing outside the provider that published them —
    /// nor, now that it is stated, outside the caller they were
    /// published to.
    pub host_name: String,
    /// How far into that volume to start, as path components
    /// relative to it.
    ///
    /// Empty mounts the volume itself, which is the common case;
    /// anything else mounts a subdirectory of it.
    ///
    /// A caller cannot escape upward with this, because the offset is
    /// components and `..` is a name rather than an instruction. The
    /// provider descends; it does not resolve.
    pub host_relative_path: Vec<String>,
    /// Where it appears inside the container, as path components from
    /// the container's root.
    ///
    /// Empty means the root itself, which a provider will almost
    /// certainly refuse — the image's own filesystem is there.
    pub container_path: Vec<String>,
}
