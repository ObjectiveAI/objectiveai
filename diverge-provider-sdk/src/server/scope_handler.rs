//! One scope, and everything that happens inside it.

use crate::endpoints::ClientRequest;

/// One request, from arrival to finish.
///
/// A [`Handler`](super::handler::Handler) reads the scope off the
/// request and hands it here; this is where the ten of them part
/// company. Below it are channels, which nothing has yet.
///
/// # It is a placeholder
///
/// The shape is deliberate and the contents are not. What a scope
/// handler needs is not settled, and three questions decide it:
///
/// **What answers a request.** Ten kinds, each with its own response
/// frame, and a provider supplies the behaviour for all of them. Some
/// arrangement of traits — one per endpoint, or one with ten methods —
/// and nothing here should guess which until one of them is written.
///
/// **How it writes.** A scope's frames go out on the connection that
/// opened it, which the handler above owns. Passing the socket down
/// works for one scope at a time and stops working the moment two run
/// at once, which they must — a laboratory streams a filetree for as
/// long as it lives, and everything else on the connection carries on
/// meanwhile. Splitting the socket and giving each scope a sender is
/// the obvious answer and is not written.
///
/// **How long it lives.** Some scopes are one exchange: a volume
/// listing answers and finishes. Some are the life of a container.
/// Both are this type, and only the second needs to be remembered — a
/// register of open scopes is what the channel frames are waiting for.
pub struct ScopeHandler {
    /// The scope this handler answers in, chosen by the client.
    scope: u32,
}

impl ScopeHandler {
    /// Take the scope a client opened.
    pub fn new(scope: u32) -> Self {
        ScopeHandler { scope }
    }

    /// The scope this handler answers in.
    pub fn scope(&self) -> u32 {
        self.scope
    }

    /// Answer one request.
    ///
    /// [`Invalid`](ClientRequest::Invalid) never arrives here — the
    /// handler above answers it, because an unreadable request needs
    /// no provider behaviour to say what went wrong.
    pub async fn handle(self, request: ClientRequest<'_>) {
        match request {
            ClientRequest::AgenticLoopRun(_) => {
                unimplemented!("an agentic loop")
            }
            ClientRequest::McpPluginRun(_) => unimplemented!("a plugin"),
            ClientRequest::LaboratoriesRun(_) => {
                unimplemented!("a laboratory")
            }
            ClientRequest::LaboratoriesConnect(_) => {
                unimplemented!("a connection")
            }
            ClientRequest::VolumesList(_) => unimplemented!("a volume listing"),
            ClientRequest::VolumesWatch(_) => unimplemented!("a watch"),
            ClientRequest::VolumesCreate(_) => {
                unimplemented!("a volume creation")
            }
            ClientRequest::VolumesEdit(_) => unimplemented!("a volume edit"),
            ClientRequest::VolumesDelete(_) => {
                unimplemented!("a volume deletion")
            }
            ClientRequest::ImagesCheck(_) => unimplemented!("an image check"),
            // Answered above, and unreachable here.
            ClientRequest::Invalid(_) => unreachable!(
                "an invalid request is answered by the connection handler"
            ),
        }
    }
}
