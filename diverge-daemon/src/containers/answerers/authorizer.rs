//! Who may join a tool from outside: the tool's exposures.

use diverge_sdk::provider::client::ConnectionAuthorizer;
use diverge_sdk::shared::containers::authorize::request::AuthorizeConnect;
use diverge_sdk::shared::containers::authorize::response::Frame;

use super::Answerer;
use crate::containers::{Key, ToolKey};
use crate::judge::key;

/// Default deny. A connect is yes when the connector's authorization
/// is an exposure's key — minted by `tools::expose`, held in memory,
/// spent by this one presentation — and the exposure is this tool's.
/// An agent container, taking no connector, is never asked and would
/// say no; a dependency tool, nobody's record, is exposed never and
/// says no.
impl ConnectionAuthorizer for Answerer {
    async fn authorize_connect(&self, request: &AuthorizeConnect) -> Frame {
        let Key::Tool(ToolKey::Record(tool)) = self.key else {
            return Frame::Denied;
        };
        if request.authorization.is_empty() {
            return Frame::Denied;
        }
        match self.daemon.live.take_exposure(&key::hash(&request.authorization)).await {
            Some(exposed) if exposed == tool => Frame::Authorized,
            _ => Frame::Denied,
        }
    }
}
