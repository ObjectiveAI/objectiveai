//! Every request, to its handler — or to the one answer there is.

use diverge_sdk::daemon::endpoints::{self, ClientRequest};
use diverge_sdk::shared::error::Error;
use diverge_sdk::wire::decode::Decode as _;
use diverge_sdk::wire::encode::{Encode, Writer};
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use super::{accounts, roles};
use crate::daemon::Daemon;
use crate::judge::Who;

/// What every request nothing serves yet is answered with.
const NOT_SERVED: &str = "not served: the daemon serves accounts and roles, and nothing else yet";

/// Read `payload` as the request that opened `scope` and hand it to
/// its handler, which answers and finishes the scope.
///
/// One arm per request of the wire, so that a request added to the
/// SDK is a request this cannot compile without. The fourteen over
/// accounts and roles have handlers; every other is answered with its
/// endpoint's own `Error`, carrying one sentence saying so, encoded as
/// the endpoint encodes it, then the finish. A payload that is no
/// request at all is finished with nothing before it, which is what
/// the wire means by a request that was not served.
pub async fn dispatch(scope: ScopeHandle, payload: &[u8], who: Who, daemon: &Daemon) {
    match ClientRequest::decode(payload).unwrap_or_else(|never| match never {}) {
        ClientRequest::AgentsCreate(_) => refused(scope, encoded(&endpoints::agents::create::server::response::Frame::Error(not_served()))).await,
        ClientRequest::AgentsGet(_) => refused(scope, encoded(&endpoints::agents::get::server::response::Frame::Error(not_served()))).await,
        ClientRequest::AgentsDelete(_) => refused(scope, encoded(&endpoints::agents::delete::server::response::Frame::Error(not_served()))).await,
        ClientRequest::AgentsMessage(_) => refused(scope, encoded(&endpoints::agents::message::server::response::Frame::Error(not_served()))).await,
        ClientRequest::AgentsLogs(_) => refused(scope, encoded(&endpoints::agents::logs::server::response::Frame::Error(not_served()))).await,
        ClientRequest::AgentsList(_) => refused(scope, encoded(&endpoints::agents::list::server::response::Frame::Error(not_served()))).await,
        ClientRequest::AgentsEdit(_) => refused(scope, encoded(&endpoints::agents::edit::server::response::Frame::Error(not_served()))).await,
        ClientRequest::AgentsTag(_) => refused(scope, encoded(&endpoints::agents::tag::server::response::Frame::Error(not_served()))).await,
        ClientRequest::AgentsUntag(_) => refused(scope, encoded(&endpoints::agents::untag::server::response::Frame::Error(not_served()))).await,
        ClientRequest::AgentsTemplatesCreate(_) => refused(scope, encoded(&endpoints::agents::templates::create::server::response::Frame::Error(not_served()))).await,
        ClientRequest::AgentsTemplatesGet(_) => refused(scope, encoded(&endpoints::agents::templates::get::server::response::Frame::Error(not_served()))).await,
        ClientRequest::AgentsTemplatesList(_) => refused(scope, encoded(&endpoints::agents::templates::list::server::response::Frame::Error(not_served()))).await,
        ClientRequest::AgentsTemplatesDelete(_) => refused(scope, encoded(&endpoints::agents::templates::delete::server::response::Frame::Error(not_served()))).await,
        ClientRequest::AgentsTemplatesTag(_) => refused(scope, encoded(&endpoints::agents::templates::tag::server::response::Frame::Error(not_served()))).await,
        ClientRequest::AgentsTemplatesUntag(_) => refused(scope, encoded(&endpoints::agents::templates::untag::server::response::Frame::Error(not_served()))).await,
        ClientRequest::ToolsCreate(_) => refused(scope, encoded(&endpoints::tools::create::server::response::Frame::Error(not_served()))).await,
        ClientRequest::ToolsGet(_) => refused(scope, encoded(&endpoints::tools::get::server::response::Frame::Error(not_served()))).await,
        ClientRequest::ToolsEdit(_) => refused(scope, encoded(&endpoints::tools::edit::server::response::Frame::Error(not_served()))).await,
        ClientRequest::ToolsConnect(_) => refused(scope, encoded(&endpoints::tools::connect::server::response::Frame::Error(not_served()))).await,
        ClientRequest::ToolsListFor(_) => refused(scope, encoded(&endpoints::tools::list_for::server::response::Frame::Error(not_served()))).await,
        ClientRequest::ToolsAttach(_) => refused(scope, encoded(&endpoints::tools::attach::server::response::Frame::Error(not_served()))).await,
        ClientRequest::ToolsDetach(_) => refused(scope, encoded(&endpoints::tools::detach::server::response::Frame::Error(not_served()))).await,
        ClientRequest::ToolsDelete(_) => refused(scope, encoded(&endpoints::tools::delete::server::response::Frame::Error(not_served()))).await,
        ClientRequest::ToolsList(_) => refused(scope, encoded(&endpoints::tools::list::server::response::Frame::Error(not_served()))).await,
        ClientRequest::ToolsTag(_) => refused(scope, encoded(&endpoints::tools::tag::server::response::Frame::Error(not_served()))).await,
        ClientRequest::ToolsUntag(_) => refused(scope, encoded(&endpoints::tools::untag::server::response::Frame::Error(not_served()))).await,
        ClientRequest::ToolsRoutesSet(_) => refused(scope, encoded(&endpoints::tools::routes::set::server::response::Frame::Error(not_served()))).await,
        ClientRequest::ToolsRoutesDelete(_) => refused(scope, encoded(&endpoints::tools::routes::delete::server::response::Frame::Error(not_served()))).await,
        ClientRequest::ToolsRoutesList(_) => refused(scope, encoded(&endpoints::tools::routes::list::server::response::Frame::Error(not_served()))).await,
        ClientRequest::ToolsTemplatesCreate(_) => refused(scope, encoded(&endpoints::tools::templates::create::server::response::Frame::Error(not_served()))).await,
        ClientRequest::ToolsTemplatesGet(_) => refused(scope, encoded(&endpoints::tools::templates::get::server::response::Frame::Error(not_served()))).await,
        ClientRequest::ToolsTemplatesList(_) => refused(scope, encoded(&endpoints::tools::templates::list::server::response::Frame::Error(not_served()))).await,
        ClientRequest::ToolsTemplatesDelete(_) => refused(scope, encoded(&endpoints::tools::templates::delete::server::response::Frame::Error(not_served()))).await,
        ClientRequest::ToolsTemplatesTag(_) => refused(scope, encoded(&endpoints::tools::templates::tag::server::response::Frame::Error(not_served()))).await,
        ClientRequest::ToolsTemplatesUntag(_) => refused(scope, encoded(&endpoints::tools::templates::untag::server::response::Frame::Error(not_served()))).await,
        ClientRequest::ResourcesUpload(_) => refused(scope, encoded(&endpoints::resources::upload::server::response::Frame::Error(not_served()))).await,
        ClientRequest::ResourcesList(_) => refused(scope, encoded(&endpoints::resources::list::server::response::Frame::Error(not_served()))).await,
        ClientRequest::ResourcesDelete(_) => refused(scope, encoded(&endpoints::resources::delete::server::response::Frame::Error(not_served()))).await,
        ClientRequest::ProvidersOutgoingAdd(_) => refused(scope, encoded(&endpoints::providers::outgoing::add::server::response::Frame::Error(not_served()))).await,
        ClientRequest::ProvidersOutgoingGet(_) => refused(scope, encoded(&endpoints::providers::outgoing::get::server::response::Frame::Error(not_served()))).await,
        ClientRequest::ProvidersOutgoingList(_) => refused(scope, encoded(&endpoints::providers::outgoing::list::server::response::Frame::Error(not_served()))).await,
        ClientRequest::ProvidersOutgoingDelete(_) => refused(scope, encoded(&endpoints::providers::outgoing::delete::server::response::Frame::Error(not_served()))).await,
        ClientRequest::ProvidersOutgoingEdit(_) => refused(scope, encoded(&endpoints::providers::outgoing::edit::server::response::Frame::Error(not_served()))).await,
        ClientRequest::ProvidersIncomingAdd(_) => refused(scope, encoded(&endpoints::providers::incoming::add::server::response::Frame::Error(not_served()))).await,
        ClientRequest::ProvidersIncomingGet(_) => refused(scope, encoded(&endpoints::providers::incoming::get::server::response::Frame::Error(not_served()))).await,
        ClientRequest::ProvidersIncomingList(_) => refused(scope, encoded(&endpoints::providers::incoming::list::server::response::Frame::Error(not_served()))).await,
        ClientRequest::ProvidersIncomingDelete(_) => refused(scope, encoded(&endpoints::providers::incoming::delete::server::response::Frame::Error(not_served()))).await,
        ClientRequest::ProvidersIncomingEdit(_) => refused(scope, encoded(&endpoints::providers::incoming::edit::server::response::Frame::Error(not_served()))).await,
        ClientRequest::AccountsCreate(frame) => accounts::create::handle(scope, frame, who, daemon).await,
        ClientRequest::AccountsGet(frame) => accounts::get::handle(scope, frame, who, daemon).await,
        ClientRequest::AccountsList(frame) => accounts::list::handle(scope, frame, who, daemon).await,
        ClientRequest::AccountsDelete(frame) => accounts::delete::handle(scope, frame, who, daemon).await,
        ClientRequest::AccountsEdit(frame) => accounts::edit::handle(scope, frame, who, daemon).await,
        ClientRequest::AccountsTag(frame) => accounts::tag::handle(scope, frame, who, daemon).await,
        ClientRequest::AccountsUntag(frame) => accounts::untag::handle(scope, frame, who, daemon).await,
        ClientRequest::RolesCreate(frame) => roles::create::handle(scope, frame, who, daemon).await,
        ClientRequest::RolesGet(frame) => roles::get::handle(scope, frame, who, daemon).await,
        ClientRequest::RolesList(frame) => roles::list::handle(scope, frame, who, daemon).await,
        ClientRequest::RolesDelete(frame) => roles::delete::handle(scope, frame, who, daemon).await,
        ClientRequest::RolesEdit(frame) => roles::edit::handle(scope, frame, who, daemon).await,
        ClientRequest::RolesTag(frame) => roles::tag::handle(scope, frame, who, daemon).await,
        ClientRequest::RolesUntag(frame) => roles::untag::handle(scope, frame, who, daemon).await,
        ClientRequest::AgentsDownload(_) => refused(scope, encoded(&endpoints::agents::download::server::response::Frame::Error(not_served()))).await,
        ClientRequest::AgentsUpload(_) => refused(scope, encoded(&endpoints::agents::upload::server::response::Frame::Error(not_served()))).await,
        ClientRequest::AgentsTransfer(_) => refused(scope, encoded(&endpoints::agents::transfer::server::response::Frame::Error(not_served()))).await,
        ClientRequest::ToolsDownload(_) => refused(scope, encoded(&endpoints::tools::download::server::response::Frame::Error(not_served()))).await,
        ClientRequest::ToolsUpload(_) => refused(scope, encoded(&endpoints::tools::upload::server::response::Frame::Error(not_served()))).await,
        ClientRequest::ToolsTransfer(_) => refused(scope, encoded(&endpoints::tools::transfer::server::response::Frame::Error(not_served()))).await,
        ClientRequest::ResourcesDownload(_) => refused(scope, encoded(&endpoints::resources::download::server::response::Frame::Error(not_served()))).await,
        ClientRequest::ResourcesTransfer(_) => refused(scope, encoded(&endpoints::resources::transfer::server::response::Frame::Error(not_served()))).await,
        ClientRequest::VolumesCreate(_) => refused(scope, encoded(&endpoints::volumes::create::server::response::Frame::Error(not_served()))).await,
        ClientRequest::VolumesGet(_) => refused(scope, encoded(&endpoints::volumes::get::server::response::Frame::Error(not_served()))).await,
        ClientRequest::VolumesList(_) => refused(scope, encoded(&endpoints::volumes::list::server::response::Frame::Error(not_served()))).await,
        ClientRequest::VolumesDelete(_) => refused(scope, encoded(&endpoints::volumes::delete::server::response::Frame::Error(not_served()))).await,
        ClientRequest::VolumesEdit(_) => refused(scope, encoded(&endpoints::volumes::edit::server::response::Frame::Error(not_served()))).await,
        ClientRequest::VolumesStat(_) => refused(scope, encoded(&endpoints::volumes::stat::server::response::Frame::Error(not_served()))).await,
        ClientRequest::VolumesDownload(_) => refused(scope, encoded(&endpoints::volumes::download::server::response::Frame::Error(not_served()))).await,
        ClientRequest::VolumesUpload(_) => refused(scope, encoded(&endpoints::volumes::upload::server::response::Frame::Error(not_served()))).await,
        ClientRequest::VolumesTransfer(_) => refused(scope, encoded(&endpoints::volumes::transfer::server::response::Frame::Error(not_served()))).await,
        ClientRequest::AgentsFiletree(_) => refused(scope, encoded(&endpoints::agents::filetree::server::response::Frame::Error(not_served()))).await,
        ClientRequest::ToolsFiletree(_) => refused(scope, encoded(&endpoints::tools::filetree::server::response::Frame::Error(not_served()))).await,
        ClientRequest::ResourcesFiletree(_) => refused(scope, encoded(&endpoints::resources::filetree::server::response::Frame::Error(not_served()))).await,
        ClientRequest::VolumesFiletree(_) => refused(scope, encoded(&endpoints::volumes::filetree::server::response::Frame::Error(not_served()))).await,
        ClientRequest::ResourcesGet(_) => refused(scope, encoded(&endpoints::resources::get::server::response::Frame::Error(not_served()))).await,
        ClientRequest::ResourcesTag(_) => refused(scope, encoded(&endpoints::resources::tag::server::response::Frame::Error(not_served()))).await,
        ClientRequest::ResourcesUntag(_) => refused(scope, encoded(&endpoints::resources::untag::server::response::Frame::Error(not_served()))).await,
        ClientRequest::PostgresGet(_) => refused(scope, encoded(&endpoints::postgres::get::server::response::Frame::Error(not_served()))).await,
        ClientRequest::PostgresConnections(_) => refused(scope, encoded(&endpoints::postgres::connections::server::response::Frame::Error(not_served()))).await,
        ClientRequest::Invalid(_) => refused(scope, None).await,
    }
}

/// The error every unserved request carries.
fn not_served() -> Error {
    Error(serde_json::Value::String(NOT_SERVED.to_string()))
}

/// Send the one refusal, if it encoded, and finish.
async fn refused(scope: ScopeHandle, response: Option<Vec<u8>>) {
    if let Some(bytes) = response {
        scope.send_response(&bytes).await;
    }
    scope.send_response_finish().await;
}

/// `frame` as its bytes, or `None` for the one way an error frame
/// cannot be written, its JSON failing.
fn encoded<F: Encode>(frame: &F) -> Option<Vec<u8>> {
    let mut bytes = Vec::new();
    frame.encode(&mut Writer::new(&mut bytes)).ok()?;
    Some(bytes)
}
