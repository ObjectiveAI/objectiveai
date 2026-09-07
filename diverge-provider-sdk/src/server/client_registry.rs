//! Pulling an image from the caller that has it.

use super::oci_stream::OciStream;
use super::scope_handle::ScopeHandle;
use crate::encode::Writer;
use crate::shared::oci;

/// How a provider asks a caller for image bytes.
///
/// Handed to
/// [`ContainerDeployer::client`](super::container_deployer::ContainerDeployer::client),
/// and only there: it is the one case where the image lives somewhere
/// the provider cannot reach, so the provider stands up a registry
/// endpoint, points its container runtime at it, and relays every
/// request the runtime makes to whoever does have the bytes.
///
/// It is the mirror of
/// `OciProxy`. That answers these
/// requests; this makes them.
///
/// # A concrete type, not a trait
///
/// Unlike everything a caller supplies. There is nothing here for a
/// provider to implement — making one of these requests means opening a
/// channel on a scope, and that is this crate's machinery. A trait
/// would be asking a provider to reimplement the one thing it cannot.
///
/// # It borrows the scope, and holds it
///
/// [`send_channel_request`](ScopeHandle::send_channel_request) needs
/// `&mut ScopeHandle`, because minting a channel advances a counter and
/// inserts into a live set. So a deploy holds the scope for as long as
/// it runs.
///
/// Which is affordable because of WHEN it runs: the container does not
/// exist yet, so there is nothing to serve on that scope, and the one
/// thing that can arrive is a client channel request — which queues in
/// an unbounded receiver and is read afterwards.
///
/// If that stops being true, the fix is the one the caller half already
/// made: [`ScopeHandle`] grows a shareable write half the way
/// [`Handle`](crate::client::handle::Handle) has one, and this holds a
/// clone instead of a borrow. Recorded here so it is a known cost
/// rather than a discovery.
///
/// # Why it is told how to wrap a request
///
/// Because the frame it goes out in belongs to the scope, not to
/// this. An
/// [`agents run`](crate::endpoints::containers::agents::run::server::channel_request::Frame::Oci)
/// and a
/// [`tools run`](crate::endpoints::containers::tools::run::server::channel_request::Frame::Oci)
/// each have their own channel-request enum with an `Oci` in it, and a
/// type serving both cannot name either — the same rule that moved
/// [`VolumeMount`](crate::shared::containers::request::VolumeMount) into
/// [`shared`](crate::shared).
///
/// The alternative was writing the tag byte here. Both endpoints
/// happen to use `0` today, and relying on that would be this module
/// reaching into two tag spaces it does not own and breaking silently
/// the day either renumbered.
#[derive(Debug)]
pub struct ClientRegistry<'a> {
    /// The scope the image is being pulled for.
    scope_handle: &'a ScopeHandle,
    /// How to put a request into this endpoint's channel-request
    /// frame.
    ///
    /// A plain `fn` rather than a closure, because there is nothing to
    /// capture: what varies between endpoints is which enum the
    /// request goes into, and that is known at the call site without
    /// any state.
    ///
    /// Written out rather than aliased, here and in
    /// [`new`](Self::new). An alias would be a second name for a
    /// signature a reader has to know anyway to supply one, and this
    /// crate keeps aliases for frames.
    wrap: fn(
        oci::request::Request<'_>,
        &mut Writer<'_>,
    ) -> Result<(), serde_json::Error>,
}

impl<'a> ClientRegistry<'a> {
    /// Point one at a scope.
    ///
    /// Public because nothing else can build one yet. When there is a
    /// handler that dispatches a server-side request, it will construct
    /// this and a provider will only ever receive one — until then a
    /// provider wires it up itself, from the [`ScopeHandle`] its
    /// [`Session`](super::session::Session) handed it and its own
    /// endpoint's frame:
    ///
    /// ```ignore
    /// ClientRegistry::new(&mut scope_handle, |request, out| {
    ///     channel_request::Frame::Oci(request).encode(out)
    /// })
    /// ```
    pub fn new(
        scope_handle: &'a ScopeHandle,
        wrap: fn(
            oci::request::Request<'_>,
            &mut Writer<'_>,
        ) -> Result<(), serde_json::Error>,
    ) -> Self {
        ClientRegistry { scope_handle, wrap }
    }

    /// Ask for one thing, and get the answer as it arrives.
    ///
    /// One channel per request, opened here and answered by the
    /// caller's `OciProxy`. What
    /// comes back is an [`OciStream`]: the registry's answer as it
    /// arrives, in as many pieces as it arrives in.
    ///
    /// # It returns before the answer does
    ///
    /// The request is written and the stream handed back; the caller is
    /// answering into it while a provider reads. Which is what a runtime
    /// pulling layers in parallel needs — each of its requests is a
    /// channel of its own, and none waits on another.
    ///
    /// # The scope segment is not added here
    ///
    /// A provider serving `/v2/<scope>/…` to its runtime strips the
    /// scope before relaying, because the caller's registry knows
    /// nothing about scopes. What arrives here is the request the
    /// caller should see.
    ///
    /// # It fails one way, and not even that one
    ///
    /// The frame either goes out or it does not. Everything after that
    /// belongs to the answer — see [`OciStream`], whose failures are
    /// this crate's plumbing and never a refusal, because a refusal is
    /// a status.
    ///
    /// The [`Err`] is unreachable in practice, and deliberately kept.
    /// A request is bytes copied into a buffer and cannot fail to
    /// encode; what the type reports is the whole endpoint frame's
    /// error, and
    /// [`agents::run`](crate::endpoints::containers::agents::run::server::channel_request::Frame)
    /// has variants that ARE JSON. Narrowing it would mean a type
    /// parameter here, which would then have to appear on
    /// [`ContainerDeployer::client`](super::container_deployer::ContainerDeployer::client)
    /// — a bound on every implementation, to delete an arm one endpoint
    /// still needs.
    pub async fn request(
        &self,
        request: oci::request::Request<'_>,
    ) -> Result<OciStream, serde_json::Error> {
        let mut payload = Vec::new();
        (self.wrap)(request, &mut Writer::new(&mut payload))?;
        let channel = self.scope_handle.send_channel_request(&payload).await;
        Ok(OciStream::new(channel))
    }
}
