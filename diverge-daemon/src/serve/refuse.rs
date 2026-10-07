//! The one answer there is: the endpoint's own error.

use diverge_sdk::daemon::endpoints::{self, ClientRequest};
use diverge_sdk::shared::error::Error;
use diverge_sdk::wire::decode::Decode as _;
use diverge_sdk::wire::encode::{Encode, Writer};
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

/// What every refusal says.
const NOT_SERVED: &str = "not served: the daemon is initialized and serves no endpoint yet";

/// Answer the request that opened `scope` with its endpoint's error,
/// and finish the scope.
///
/// Every one of the eighty-nine requests has an answer type with an
/// `Error` variant, and this is that variant, carrying one sentence
/// saying the daemon serves nothing yet, encoded as the endpoint
/// encodes it — the one response, then the finish. A payload that is no request at all is
/// finished with nothing before it, which is what the wire means by a
/// request that was not served. `account` is who asked, admitted by
/// [`judge`](super::judge); nothing is judged by it yet, and every
/// handler that replaces an arm here will be.
pub async fn refuse(scope: ScopeHandle, payload: &[u8], account: &str) {
    let _ = account;
    let error = Error(serde_json::Value::String(NOT_SERVED.to_string()));
    let response = match ClientRequest::decode(payload).unwrap_or_else(|never| match never {}) {
        ClientRequest::AgentsCreate(_) => encoded(&endpoints::agents::create::server::response::Frame::Error(error)),
        ClientRequest::AgentsGet(_) => encoded(&endpoints::agents::get::server::response::Frame::Error(error)),
        ClientRequest::AgentsDelete(_) => encoded(&endpoints::agents::delete::server::response::Frame::Error(error)),
        ClientRequest::AgentsMessage(_) => encoded(&endpoints::agents::message::server::response::Frame::Error(error)),
        ClientRequest::AgentsLogs(_) => encoded(&endpoints::agents::logs::server::response::Frame::Error(error)),
        ClientRequest::AgentsList(_) => encoded(&endpoints::agents::list::server::response::Frame::Error(error)),
        ClientRequest::AgentsEdit(_) => encoded(&endpoints::agents::edit::server::response::Frame::Error(error)),
        ClientRequest::AgentsTag(_) => encoded(&endpoints::agents::tag::server::response::Frame::Error(error)),
        ClientRequest::AgentsUntag(_) => encoded(&endpoints::agents::untag::server::response::Frame::Error(error)),
        ClientRequest::AgentsTemplatesCreate(_) => encoded(&endpoints::agents::templates::create::server::response::Frame::Error(error)),
        ClientRequest::AgentsTemplatesGet(_) => encoded(&endpoints::agents::templates::get::server::response::Frame::Error(error)),
        ClientRequest::AgentsTemplatesList(_) => encoded(&endpoints::agents::templates::list::server::response::Frame::Error(error)),
        ClientRequest::AgentsTemplatesDelete(_) => encoded(&endpoints::agents::templates::delete::server::response::Frame::Error(error)),
        ClientRequest::AgentsTemplatesTag(_) => encoded(&endpoints::agents::templates::tag::server::response::Frame::Error(error)),
        ClientRequest::AgentsTemplatesUntag(_) => encoded(&endpoints::agents::templates::untag::server::response::Frame::Error(error)),
        ClientRequest::ToolsCreate(_) => encoded(&endpoints::tools::create::server::response::Frame::Error(error)),
        ClientRequest::ToolsGet(_) => encoded(&endpoints::tools::get::server::response::Frame::Error(error)),
        ClientRequest::ToolsEdit(_) => encoded(&endpoints::tools::edit::server::response::Frame::Error(error)),
        ClientRequest::ToolsConnect(_) => encoded(&endpoints::tools::connect::server::response::Frame::Error(error)),
        ClientRequest::ToolsListFor(_) => encoded(&endpoints::tools::list_for::server::response::Frame::Error(error)),
        ClientRequest::ToolsAttach(_) => encoded(&endpoints::tools::attach::server::response::Frame::Error(error)),
        ClientRequest::ToolsDetach(_) => encoded(&endpoints::tools::detach::server::response::Frame::Error(error)),
        ClientRequest::ToolsDelete(_) => encoded(&endpoints::tools::delete::server::response::Frame::Error(error)),
        ClientRequest::ToolsList(_) => encoded(&endpoints::tools::list::server::response::Frame::Error(error)),
        ClientRequest::ToolsTag(_) => encoded(&endpoints::tools::tag::server::response::Frame::Error(error)),
        ClientRequest::ToolsUntag(_) => encoded(&endpoints::tools::untag::server::response::Frame::Error(error)),
        ClientRequest::ToolsRoutesSet(_) => encoded(&endpoints::tools::routes::set::server::response::Frame::Error(error)),
        ClientRequest::ToolsRoutesDelete(_) => encoded(&endpoints::tools::routes::delete::server::response::Frame::Error(error)),
        ClientRequest::ToolsRoutesList(_) => encoded(&endpoints::tools::routes::list::server::response::Frame::Error(error)),
        ClientRequest::ToolsTemplatesCreate(_) => encoded(&endpoints::tools::templates::create::server::response::Frame::Error(error)),
        ClientRequest::ToolsTemplatesGet(_) => encoded(&endpoints::tools::templates::get::server::response::Frame::Error(error)),
        ClientRequest::ToolsTemplatesList(_) => encoded(&endpoints::tools::templates::list::server::response::Frame::Error(error)),
        ClientRequest::ToolsTemplatesDelete(_) => encoded(&endpoints::tools::templates::delete::server::response::Frame::Error(error)),
        ClientRequest::ToolsTemplatesTag(_) => encoded(&endpoints::tools::templates::tag::server::response::Frame::Error(error)),
        ClientRequest::ToolsTemplatesUntag(_) => encoded(&endpoints::tools::templates::untag::server::response::Frame::Error(error)),
        ClientRequest::ResourcesUpload(_) => encoded(&endpoints::resources::upload::server::response::Frame::Error(error)),
        ClientRequest::ResourcesList(_) => encoded(&endpoints::resources::list::server::response::Frame::Error(error)),
        ClientRequest::ResourcesDelete(_) => encoded(&endpoints::resources::delete::server::response::Frame::Error(error)),
        ClientRequest::ProvidersOutgoingAdd(_) => encoded(&endpoints::providers::outgoing::add::server::response::Frame::Error(error)),
        ClientRequest::ProvidersOutgoingGet(_) => encoded(&endpoints::providers::outgoing::get::server::response::Frame::Error(error)),
        ClientRequest::ProvidersOutgoingList(_) => encoded(&endpoints::providers::outgoing::list::server::response::Frame::Error(error)),
        ClientRequest::ProvidersOutgoingDelete(_) => encoded(&endpoints::providers::outgoing::delete::server::response::Frame::Error(error)),
        ClientRequest::ProvidersOutgoingEdit(_) => encoded(&endpoints::providers::outgoing::edit::server::response::Frame::Error(error)),
        ClientRequest::ProvidersIncomingAdd(_) => encoded(&endpoints::providers::incoming::add::server::response::Frame::Error(error)),
        ClientRequest::ProvidersIncomingGet(_) => encoded(&endpoints::providers::incoming::get::server::response::Frame::Error(error)),
        ClientRequest::ProvidersIncomingList(_) => encoded(&endpoints::providers::incoming::list::server::response::Frame::Error(error)),
        ClientRequest::ProvidersIncomingDelete(_) => encoded(&endpoints::providers::incoming::delete::server::response::Frame::Error(error)),
        ClientRequest::ProvidersIncomingEdit(_) => encoded(&endpoints::providers::incoming::edit::server::response::Frame::Error(error)),
        ClientRequest::AccountsCreate(_) => encoded(&endpoints::accounts::create::server::response::Frame::Error(error)),
        ClientRequest::AccountsGet(_) => encoded(&endpoints::accounts::get::server::response::Frame::Error(error)),
        ClientRequest::AccountsList(_) => encoded(&endpoints::accounts::list::server::response::Frame::Error(error)),
        ClientRequest::AccountsDelete(_) => encoded(&endpoints::accounts::delete::server::response::Frame::Error(error)),
        ClientRequest::AccountsEdit(_) => encoded(&endpoints::accounts::edit::server::response::Frame::Error(error)),
        ClientRequest::AccountsTag(_) => encoded(&endpoints::accounts::tag::server::response::Frame::Error(error)),
        ClientRequest::AccountsUntag(_) => encoded(&endpoints::accounts::untag::server::response::Frame::Error(error)),
        ClientRequest::RolesCreate(_) => encoded(&endpoints::roles::create::server::response::Frame::Error(error)),
        ClientRequest::RolesGet(_) => encoded(&endpoints::roles::get::server::response::Frame::Error(error)),
        ClientRequest::RolesList(_) => encoded(&endpoints::roles::list::server::response::Frame::Error(error)),
        ClientRequest::RolesDelete(_) => encoded(&endpoints::roles::delete::server::response::Frame::Error(error)),
        ClientRequest::RolesEdit(_) => encoded(&endpoints::roles::edit::server::response::Frame::Error(error)),
        ClientRequest::RolesTag(_) => encoded(&endpoints::roles::tag::server::response::Frame::Error(error)),
        ClientRequest::RolesUntag(_) => encoded(&endpoints::roles::untag::server::response::Frame::Error(error)),
        ClientRequest::AgentsDownload(_) => encoded(&endpoints::agents::download::server::response::Frame::Error(error)),
        ClientRequest::AgentsUpload(_) => encoded(&endpoints::agents::upload::server::response::Frame::Error(error)),
        ClientRequest::AgentsTransfer(_) => encoded(&endpoints::agents::transfer::server::response::Frame::Error(error)),
        ClientRequest::ToolsDownload(_) => encoded(&endpoints::tools::download::server::response::Frame::Error(error)),
        ClientRequest::ToolsUpload(_) => encoded(&endpoints::tools::upload::server::response::Frame::Error(error)),
        ClientRequest::ToolsTransfer(_) => encoded(&endpoints::tools::transfer::server::response::Frame::Error(error)),
        ClientRequest::ResourcesDownload(_) => encoded(&endpoints::resources::download::server::response::Frame::Error(error)),
        ClientRequest::ResourcesTransfer(_) => encoded(&endpoints::resources::transfer::server::response::Frame::Error(error)),
        ClientRequest::VolumesCreate(_) => encoded(&endpoints::volumes::create::server::response::Frame::Error(error)),
        ClientRequest::VolumesGet(_) => encoded(&endpoints::volumes::get::server::response::Frame::Error(error)),
        ClientRequest::VolumesList(_) => encoded(&endpoints::volumes::list::server::response::Frame::Error(error)),
        ClientRequest::VolumesDelete(_) => encoded(&endpoints::volumes::delete::server::response::Frame::Error(error)),
        ClientRequest::VolumesEdit(_) => encoded(&endpoints::volumes::edit::server::response::Frame::Error(error)),
        ClientRequest::VolumesStat(_) => encoded(&endpoints::volumes::stat::server::response::Frame::Error(error)),
        ClientRequest::VolumesDownload(_) => encoded(&endpoints::volumes::download::server::response::Frame::Error(error)),
        ClientRequest::VolumesUpload(_) => encoded(&endpoints::volumes::upload::server::response::Frame::Error(error)),
        ClientRequest::VolumesTransfer(_) => encoded(&endpoints::volumes::transfer::server::response::Frame::Error(error)),
        ClientRequest::AgentsFiletree(_) => encoded(&endpoints::agents::filetree::server::response::Frame::Error(error)),
        ClientRequest::ToolsFiletree(_) => encoded(&endpoints::tools::filetree::server::response::Frame::Error(error)),
        ClientRequest::ResourcesFiletree(_) => encoded(&endpoints::resources::filetree::server::response::Frame::Error(error)),
        ClientRequest::VolumesFiletree(_) => encoded(&endpoints::volumes::filetree::server::response::Frame::Error(error)),
        ClientRequest::ResourcesGet(_) => encoded(&endpoints::resources::get::server::response::Frame::Error(error)),
        ClientRequest::ResourcesTag(_) => encoded(&endpoints::resources::tag::server::response::Frame::Error(error)),
        ClientRequest::ResourcesUntag(_) => encoded(&endpoints::resources::untag::server::response::Frame::Error(error)),
        ClientRequest::PostgresGet(_) => encoded(&endpoints::postgres::get::server::response::Frame::Error(error)),
        ClientRequest::PostgresSet(_) => encoded(&endpoints::postgres::set::server::response::Frame::Error(error)),
        ClientRequest::PostgresConnections(_) => encoded(&endpoints::postgres::connections::server::response::Frame::Error(error)),
        ClientRequest::Invalid(_) => None,
    };
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
