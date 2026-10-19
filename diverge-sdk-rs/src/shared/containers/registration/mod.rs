//! Registering a container with its caller: what its program declared
//! at registration, asked of the caller, and the first values back.
//!
//! A container's program answers its registration with two things:
//! the tools it depends on — each a
//! [`dependencies::Template`](super::dependencies::Template), an
//! agent's alone — and the secrets it takes, each by name to its
//! mode, a [`secrets::Declaration`](super::secrets::Declaration), an
//! agent's or a tool's. The proxy carries both on the begin's `Begun`;
//! the provider, when either is non-empty, opens one channel on the
//! run scope with [`request::Request`] — both verbatim, with the run's
//! id — and the caller answers with one [`response::Frame`]: deployed,
//! with the first values of the secrets, or an error in its own words;
//! then the channel finishes. The provider feeds the values to the
//! container on its begin's own `secrets` channel, awaits the feed,
//! and only then sends the id. The run ends, the container stopped,
//! on anything but a deploy. A program that declares neither is never
//! asked about, and its id follows at once.
//!
//! What the caller does with the ask is its own: it validates the
//! declaration against the template the container was made from —
//! for a dependency, against its dependency template, which was
//! already checked against the agent's — deploys each dependency,
//! resolves each name through the record's binding, locks a free
//! value for every `rotating` name and reads one for every `fixed`,
//! and answers. The provider carries and does not read any of it.
//! Later activations of the same container take no new registration:
//! the caller feeds them on the run's own `secrets` channel, and the
//! provider relays each to the begin's.

pub mod request;
pub mod response;
