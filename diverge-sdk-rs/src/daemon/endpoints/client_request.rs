//! Every request a client can open a scope with.

use std::convert::Infallible;
use std::fmt;

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

use super::{accounts, agents, providers, resources, roles, tools, volumes};

/// The payload of a
/// [`Request`](crate::wire::frame::client::ClientFrame::Request).
///
/// One variant per scope, in tag order, plus
/// [`Invalid`](Self::Invalid). This is the only place the tag values
/// meet: each request states its own in its own module, and nothing
/// else can see all of them at once — so this is what the table in
/// [`endpoints`](super) is a table OF.
///
/// # Decoding cannot fail
///
/// There is no error type. A payload this does not understand — an
/// unknown tag, a body that will not parse, no bytes at all — becomes
/// [`Invalid`](Self::Invalid) rather than a failure, so the daemon
/// always has a request to answer and a malformed one is a scope like
/// any other. The alternative, refusing to decode, would leave the
/// daemon holding bytes it could not name and no scope to complain in.
#[derive(Debug, Clone, PartialEq)]
pub enum ClientRequest<'a> {
    /// Tag `0`. Create an agent under a name.
    AgentsCreate(agents::create::client::request::Frame),
    /// Tag `1`. Get one agent.
    AgentsGet(agents::get::client::request::Frame),
    /// Tag `2`. Delete an agent.
    AgentsDelete(agents::delete::client::request::Frame),
    /// Tag `3`. Send an agent a message.
    AgentsMessage(agents::message::client::request::Frame),
    /// Tag `4`. Read an agent's log, filtered, and perhaps kept open.
    AgentsLogs(agents::logs::client::request::Frame),
    /// Tag `5`. List the caller's agents, narrowed.
    AgentsList(agents::list::client::request::Frame),
    /// Tag `6`. Change an agent.
    AgentsEdit(agents::edit::client::request::Frame),
    /// Tag `7`. Put tags on an agent.
    AgentsTag(agents::tag::client::request::Frame),
    /// Tag `8`. Take tags off an agent.
    AgentsUntag(agents::untag::client::request::Frame),
    /// Tag `9`. Make a template.
    AgentsTemplatesCreate(agents::templates::create::client::request::Frame),
    /// Tag `10`. Get one template by id.
    AgentsTemplatesGet(agents::templates::get::client::request::Frame),
    /// Tag `11`. List the caller's templates, narrowed.
    AgentsTemplatesList(agents::templates::list::client::request::Frame),
    /// Tag `12`. Delete a template by id.
    AgentsTemplatesDelete(agents::templates::delete::client::request::Frame),
    /// Tag `13`. Put tags on a template.
    AgentsTemplatesTag(agents::templates::tag::client::request::Frame),
    /// Tag `14`. Take tags off a template.
    AgentsTemplatesUntag(agents::templates::untag::client::request::Frame),
    /// Tag `15`. Create a tool under a name.
    ToolsCreate(tools::create::client::request::Frame),
    /// Tag `16`. Get one tool.
    ToolsGet(tools::get::client::request::Frame),
    /// Tag `17`. Change a tool.
    ToolsEdit(tools::edit::client::request::Frame),
    /// Tag `18`. Hold somebody else's tool container under a name.
    ToolsConnect(tools::connect::client::request::Frame),
    /// Tag `19`. Ask a provider which tool containers an identity runs.
    ToolsListFor(tools::list_for::client::request::Frame),
    /// Tag `20`. Attach a tool to an agent.
    ToolsAttach(tools::attach::client::request::Frame),
    /// Tag `21`. Detach a tool from an agent.
    ToolsDetach(tools::detach::client::request::Frame),
    /// Tag `22`. Delete a tool.
    ToolsDelete(tools::delete::client::request::Frame),
    /// Tag `23`. List the caller's tools, narrowed.
    ToolsList(tools::list::client::request::Frame),
    /// Tag `24`. Put tags on a tool.
    ToolsTag(tools::tag::client::request::Frame),
    /// Tag `25`. Take tags off a tool.
    ToolsUntag(tools::untag::client::request::Frame),
    /// Tag `26`. Set a dependency position's route to a tool.
    ToolsRoutesSet(tools::routes::set::client::request::Frame),
    /// Tag `27`. Take a route up.
    ToolsRoutesDelete(tools::routes::delete::client::request::Frame),
    /// Tag `28`. List the caller's routes, narrowed.
    ToolsRoutesList(tools::routes::list::client::request::Frame),
    /// Tag `29`. Make a tool template.
    ToolsTemplatesCreate(tools::templates::create::client::request::Frame),
    /// Tag `30`. Get one tool template by id.
    ToolsTemplatesGet(tools::templates::get::client::request::Frame),
    /// Tag `31`. List the caller's tool templates, narrowed.
    ToolsTemplatesList(tools::templates::list::client::request::Frame),
    /// Tag `32`. Delete a tool template by id.
    ToolsTemplatesDelete(tools::templates::delete::client::request::Frame),
    /// Tag `33`. Put tags on a tool template.
    ToolsTemplatesTag(tools::templates::tag::client::request::Frame),
    /// Tag `34`. Take tags off a tool template.
    ToolsTemplatesUntag(tools::templates::untag::client::request::Frame),
    /// Tag `35`. Upload a file or a directory.
    ResourcesUpload(resources::upload::client::request::Frame),
    /// Tag `36`. List the resources, narrowed.
    ResourcesList(resources::list::client::request::Frame),
    /// Tag `37`. Delete a resource by id.
    ResourcesDelete(resources::delete::client::request::Frame),
    /// Tag `38`. Add a provider to dial.
    ProvidersOutgoingAdd(providers::outgoing::add::client::request::Frame),
    /// Tag `39`. Get one outgoing provider.
    ProvidersOutgoingGet(providers::outgoing::get::client::request::Frame),
    /// Tag `40`. List the caller's outgoing providers, narrowed.
    ProvidersOutgoingList(providers::outgoing::list::client::request::Frame),
    /// Tag `41`. Forget an outgoing provider.
    ProvidersOutgoingDelete(providers::outgoing::delete::client::request::Frame),
    /// Tag `42`. Replace an outgoing provider's mode.
    ProvidersOutgoingEdit(providers::outgoing::edit::client::request::Frame),
    /// Tag `43`. Add a judge of incoming providers.
    ProvidersIncomingAdd(providers::incoming::add::client::request::Frame),
    /// Tag `44`. Get one judge.
    ProvidersIncomingGet(providers::incoming::get::client::request::Frame),
    /// Tag `45`. List the caller's judges, narrowed.
    ProvidersIncomingList(providers::incoming::list::client::request::Frame),
    /// Tag `46`. Take a judge out.
    ProvidersIncomingDelete(providers::incoming::delete::client::request::Frame),
    /// Tag `47`. Replace a judge.
    ProvidersIncomingEdit(providers::incoming::edit::client::request::Frame),
    /// Tag `48`. Create an account.
    AccountsCreate(accounts::create::client::request::Frame),
    /// Tag `49`. Get one account.
    AccountsGet(accounts::get::client::request::Frame),
    /// Tag `50`. List the accounts, narrowed.
    AccountsList(accounts::list::client::request::Frame),
    /// Tag `51`. Delete an account.
    AccountsDelete(accounts::delete::client::request::Frame),
    /// Tag `52`. Change an account.
    AccountsEdit(accounts::edit::client::request::Frame),
    /// Tag `53`. Put tags on an account.
    AccountsTag(accounts::tag::client::request::Frame),
    /// Tag `54`. Take tags off an account.
    AccountsUntag(accounts::untag::client::request::Frame),
    /// Tag `55`. Create a role.
    RolesCreate(roles::create::client::request::Frame),
    /// Tag `56`. Get one role.
    RolesGet(roles::get::client::request::Frame),
    /// Tag `57`. List the roles, narrowed.
    RolesList(roles::list::client::request::Frame),
    /// Tag `58`. Delete a role.
    RolesDelete(roles::delete::client::request::Frame),
    /// Tag `59`. Change a role.
    RolesEdit(roles::edit::client::request::Frame),
    /// Tag `60`. Put tags on a role.
    RolesTag(roles::tag::client::request::Frame),
    /// Tag `61`. Take tags off a role.
    RolesUntag(roles::untag::client::request::Frame),
    /// Tag `62`. Send the client files out of an agent's container.
    AgentsDownload(agents::download::client::request::Frame),
    /// Tag `63`. Put files into an agent's container.
    AgentsUpload(agents::upload::client::request::Frame),
    /// Tag `64`. Copy files out of an agent's container elsewhere.
    AgentsTransfer(agents::transfer::client::request::Frame),
    /// Tag `65`. Send the client files out of a tool's container.
    ToolsDownload(tools::download::client::request::Frame),
    /// Tag `66`. Put files into a tool's container.
    ToolsUpload(tools::upload::client::request::Frame),
    /// Tag `67`. Copy files out of a tool's container elsewhere.
    ToolsTransfer(tools::transfer::client::request::Frame),
    /// Tag `68`. Send the client a resource, or a part of one.
    ResourcesDownload(resources::download::client::request::Frame),
    /// Tag `69`. Copy a resource, or a part of one, elsewhere.
    ResourcesTransfer(resources::transfer::client::request::Frame),
    /// Tag `70`. Create a volume on a provider.
    VolumesCreate(volumes::create::client::request::Frame),
    /// Tag `71`. Get one volume.
    VolumesGet(volumes::get::client::request::Frame),
    /// Tag `72`. List the volumes, narrowed.
    VolumesList(volumes::list::client::request::Frame),
    /// Tag `73`. Delete a volume.
    VolumesDelete(volumes::delete::client::request::Frame),
    /// Tag `74`. Change a volume's size or mode.
    VolumesEdit(volumes::edit::client::request::Frame),
    /// Tag `75`. Walk a volume for its use and its hash.
    VolumesStat(volumes::stat::client::request::Frame),
    /// Tag `76`. Send the client files out of a volume.
    VolumesDownload(volumes::download::client::request::Frame),
    /// Tag `77`. Put files into a volume.
    VolumesUpload(volumes::upload::client::request::Frame),
    /// Tag `78`. Copy files out of a volume elsewhere.
    VolumesTransfer(volumes::transfer::client::request::Frame),
    /// Tag `79`. Watch an agent's container whole.
    AgentsFiletree(agents::filetree::client::request::Frame),
    /// Tag `80`. Watch a tool's container whole.
    ToolsFiletree(tools::filetree::client::request::Frame),
    /// Tag `81`. See a directory resource's tree, once.
    ResourcesFiletree(resources::filetree::client::request::Frame),
    /// Tag `82`. See a volume's tree, once.
    VolumesFiletree(volumes::filetree::client::request::Frame),
    /// Tag `83`. Get one resource by id.
    ResourcesGet(resources::get::client::request::Frame),
    /// Tag `84`. Put tags on a resource.
    ResourcesTag(resources::tag::client::request::Frame),
    /// Tag `85`. Take tags off a resource.
    ResourcesUntag(resources::untag::client::request::Frame),
    /// Something this version cannot read, kept as it arrived.
    ///
    /// No tag of its own. It is not a request a client sends — it is
    /// what a request BECOMES when the daemon cannot make one of the
    /// others out of it, so it carries the payload verbatim and
    /// encodes back to exactly those bytes. The daemon finishes the
    /// scope over it with nothing in front: a finish that no response
    /// precedes is already what the wire means by a request that
    /// could not be served, and it names no endpoint's error
    /// vocabulary, which an invalid request has none of.
    Invalid(&'a [u8]),
}

impl Encode for ClientRequest<'_> {
    /// The ordinary JSON failure: every request is JSON after its
    /// tag.
    type Error = serde_json::Error;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), serde_json::Error> {
        match self {
            ClientRequest::AgentsCreate(frame) => frame.encode(out),
            ClientRequest::AgentsGet(frame) => frame.encode(out),
            ClientRequest::AgentsDelete(frame) => frame.encode(out),
            ClientRequest::AgentsMessage(frame) => frame.encode(out),
            ClientRequest::AgentsLogs(frame) => frame.encode(out),
            ClientRequest::AgentsList(frame) => frame.encode(out),
            ClientRequest::AgentsEdit(frame) => frame.encode(out),
            ClientRequest::AgentsTag(frame) => frame.encode(out),
            ClientRequest::AgentsUntag(frame) => frame.encode(out),
            ClientRequest::AgentsTemplatesCreate(frame) => frame.encode(out),
            ClientRequest::AgentsTemplatesGet(frame) => frame.encode(out),
            ClientRequest::AgentsTemplatesList(frame) => frame.encode(out),
            ClientRequest::AgentsTemplatesDelete(frame) => frame.encode(out),
            ClientRequest::AgentsTemplatesTag(frame) => frame.encode(out),
            ClientRequest::AgentsTemplatesUntag(frame) => frame.encode(out),
            ClientRequest::ToolsCreate(frame) => frame.encode(out),
            ClientRequest::ToolsGet(frame) => frame.encode(out),
            ClientRequest::ToolsEdit(frame) => frame.encode(out),
            ClientRequest::ToolsConnect(frame) => frame.encode(out),
            ClientRequest::ToolsListFor(frame) => frame.encode(out),
            ClientRequest::ToolsAttach(frame) => frame.encode(out),
            ClientRequest::ToolsDetach(frame) => frame.encode(out),
            ClientRequest::ToolsDelete(frame) => frame.encode(out),
            ClientRequest::ToolsList(frame) => frame.encode(out),
            ClientRequest::ToolsTag(frame) => frame.encode(out),
            ClientRequest::ToolsUntag(frame) => frame.encode(out),
            ClientRequest::ToolsRoutesSet(frame) => frame.encode(out),
            ClientRequest::ToolsRoutesDelete(frame) => frame.encode(out),
            ClientRequest::ToolsRoutesList(frame) => frame.encode(out),
            ClientRequest::ToolsTemplatesCreate(frame) => frame.encode(out),
            ClientRequest::ToolsTemplatesGet(frame) => frame.encode(out),
            ClientRequest::ToolsTemplatesList(frame) => frame.encode(out),
            ClientRequest::ToolsTemplatesDelete(frame) => frame.encode(out),
            ClientRequest::ToolsTemplatesTag(frame) => frame.encode(out),
            ClientRequest::ToolsTemplatesUntag(frame) => frame.encode(out),
            ClientRequest::ResourcesUpload(frame) => frame.encode(out),
            ClientRequest::ResourcesList(frame) => frame.encode(out),
            ClientRequest::ResourcesDelete(frame) => frame.encode(out),
            ClientRequest::ProvidersOutgoingAdd(frame) => frame.encode(out),
            ClientRequest::ProvidersOutgoingGet(frame) => frame.encode(out),
            ClientRequest::ProvidersOutgoingList(frame) => frame.encode(out),
            ClientRequest::ProvidersOutgoingDelete(frame) => frame.encode(out),
            ClientRequest::ProvidersOutgoingEdit(frame) => frame.encode(out),
            ClientRequest::ProvidersIncomingAdd(frame) => frame.encode(out),
            ClientRequest::ProvidersIncomingGet(frame) => frame.encode(out),
            ClientRequest::ProvidersIncomingList(frame) => frame.encode(out),
            ClientRequest::ProvidersIncomingDelete(frame) => frame.encode(out),
            ClientRequest::ProvidersIncomingEdit(frame) => frame.encode(out),
            ClientRequest::AccountsCreate(frame) => frame.encode(out),
            ClientRequest::AccountsGet(frame) => frame.encode(out),
            ClientRequest::AccountsList(frame) => frame.encode(out),
            ClientRequest::AccountsDelete(frame) => frame.encode(out),
            ClientRequest::AccountsEdit(frame) => frame.encode(out),
            ClientRequest::AccountsTag(frame) => frame.encode(out),
            ClientRequest::AccountsUntag(frame) => frame.encode(out),
            ClientRequest::RolesCreate(frame) => frame.encode(out),
            ClientRequest::RolesGet(frame) => frame.encode(out),
            ClientRequest::RolesList(frame) => frame.encode(out),
            ClientRequest::RolesDelete(frame) => frame.encode(out),
            ClientRequest::RolesEdit(frame) => frame.encode(out),
            ClientRequest::RolesTag(frame) => frame.encode(out),
            ClientRequest::RolesUntag(frame) => frame.encode(out),
            ClientRequest::AgentsDownload(frame) => frame.encode(out),
            ClientRequest::AgentsUpload(frame) => frame.encode(out),
            ClientRequest::AgentsTransfer(frame) => frame.encode(out),
            ClientRequest::ToolsDownload(frame) => frame.encode(out),
            ClientRequest::ToolsUpload(frame) => frame.encode(out),
            ClientRequest::ToolsTransfer(frame) => frame.encode(out),
            ClientRequest::ResourcesDownload(frame) => frame.encode(out),
            ClientRequest::ResourcesTransfer(frame) => frame.encode(out),
            ClientRequest::VolumesCreate(frame) => frame.encode(out),
            ClientRequest::VolumesGet(frame) => frame.encode(out),
            ClientRequest::VolumesList(frame) => frame.encode(out),
            ClientRequest::VolumesDelete(frame) => frame.encode(out),
            ClientRequest::VolumesEdit(frame) => frame.encode(out),
            ClientRequest::VolumesStat(frame) => frame.encode(out),
            ClientRequest::VolumesDownload(frame) => frame.encode(out),
            ClientRequest::VolumesUpload(frame) => frame.encode(out),
            ClientRequest::VolumesTransfer(frame) => frame.encode(out),
            ClientRequest::AgentsFiletree(frame) => frame.encode(out),
            ClientRequest::ToolsFiletree(frame) => frame.encode(out),
            ClientRequest::ResourcesFiletree(frame) => frame.encode(out),
            ClientRequest::VolumesFiletree(frame) => frame.encode(out),
            ClientRequest::ResourcesGet(frame) => frame.encode(out),
            ClientRequest::ResourcesTag(frame) => frame.encode(out),
            ClientRequest::ResourcesUntag(frame) => frame.encode(out),
            ClientRequest::Invalid(bytes) => {
                out.extend_from_slice(bytes);
                Ok(())
            }
        }
    }
}

/// Dispatch on the tag, and fall back to
/// [`Invalid`](ClientRequest::Invalid) rather than refusing.
///
/// Each request is handed the WHOLE payload, tag included, because
/// each checks its own tag — this reads the byte to choose a decoder
/// and does not strip it.
impl<'a> Decode<'a> for ClientRequest<'a> {
    /// [`Infallible`]: a payload that cannot be read is a request all
    /// the same.
    type Error = Infallible;

    fn decode(bytes: &'a [u8]) -> Result<Self, Infallible> {
        let Some(tag) = bytes.first() else {
            return Ok(ClientRequest::Invalid(bytes));
        };
        let request = match *tag {
            0 => agents::create::client::request::Frame::decode(bytes)
                .map(ClientRequest::AgentsCreate)
                .ok(),
            1 => agents::get::client::request::Frame::decode(bytes)
                .map(ClientRequest::AgentsGet)
                .ok(),
            2 => agents::delete::client::request::Frame::decode(bytes)
                .map(ClientRequest::AgentsDelete)
                .ok(),
            3 => agents::message::client::request::Frame::decode(bytes)
                .map(ClientRequest::AgentsMessage)
                .ok(),
            4 => agents::logs::client::request::Frame::decode(bytes)
                .map(ClientRequest::AgentsLogs)
                .ok(),
            5 => agents::list::client::request::Frame::decode(bytes)
                .map(ClientRequest::AgentsList)
                .ok(),
            6 => agents::edit::client::request::Frame::decode(bytes)
                .map(ClientRequest::AgentsEdit)
                .ok(),
            7 => agents::tag::client::request::Frame::decode(bytes)
                .map(ClientRequest::AgentsTag)
                .ok(),
            8 => agents::untag::client::request::Frame::decode(bytes)
                .map(ClientRequest::AgentsUntag)
                .ok(),
            9 => agents::templates::create::client::request::Frame::decode(bytes)
                .map(ClientRequest::AgentsTemplatesCreate)
                .ok(),
            10 => agents::templates::get::client::request::Frame::decode(bytes)
                .map(ClientRequest::AgentsTemplatesGet)
                .ok(),
            11 => agents::templates::list::client::request::Frame::decode(bytes)
                .map(ClientRequest::AgentsTemplatesList)
                .ok(),
            12 => agents::templates::delete::client::request::Frame::decode(bytes)
                .map(ClientRequest::AgentsTemplatesDelete)
                .ok(),
            13 => agents::templates::tag::client::request::Frame::decode(bytes)
                .map(ClientRequest::AgentsTemplatesTag)
                .ok(),
            14 => agents::templates::untag::client::request::Frame::decode(bytes)
                .map(ClientRequest::AgentsTemplatesUntag)
                .ok(),
            15 => tools::create::client::request::Frame::decode(bytes)
                .map(ClientRequest::ToolsCreate)
                .ok(),
            16 => tools::get::client::request::Frame::decode(bytes)
                .map(ClientRequest::ToolsGet)
                .ok(),
            17 => tools::edit::client::request::Frame::decode(bytes)
                .map(ClientRequest::ToolsEdit)
                .ok(),
            18 => tools::connect::client::request::Frame::decode(bytes)
                .map(ClientRequest::ToolsConnect)
                .ok(),
            19 => tools::list_for::client::request::Frame::decode(bytes)
                .map(ClientRequest::ToolsListFor)
                .ok(),
            20 => tools::attach::client::request::Frame::decode(bytes)
                .map(ClientRequest::ToolsAttach)
                .ok(),
            21 => tools::detach::client::request::Frame::decode(bytes)
                .map(ClientRequest::ToolsDetach)
                .ok(),
            22 => tools::delete::client::request::Frame::decode(bytes)
                .map(ClientRequest::ToolsDelete)
                .ok(),
            23 => tools::list::client::request::Frame::decode(bytes)
                .map(ClientRequest::ToolsList)
                .ok(),
            24 => tools::tag::client::request::Frame::decode(bytes)
                .map(ClientRequest::ToolsTag)
                .ok(),
            25 => tools::untag::client::request::Frame::decode(bytes)
                .map(ClientRequest::ToolsUntag)
                .ok(),
            26 => tools::routes::set::client::request::Frame::decode(bytes)
                .map(ClientRequest::ToolsRoutesSet)
                .ok(),
            27 => tools::routes::delete::client::request::Frame::decode(bytes)
                .map(ClientRequest::ToolsRoutesDelete)
                .ok(),
            28 => tools::routes::list::client::request::Frame::decode(bytes)
                .map(ClientRequest::ToolsRoutesList)
                .ok(),
            29 => tools::templates::create::client::request::Frame::decode(bytes)
                .map(ClientRequest::ToolsTemplatesCreate)
                .ok(),
            30 => tools::templates::get::client::request::Frame::decode(bytes)
                .map(ClientRequest::ToolsTemplatesGet)
                .ok(),
            31 => tools::templates::list::client::request::Frame::decode(bytes)
                .map(ClientRequest::ToolsTemplatesList)
                .ok(),
            32 => tools::templates::delete::client::request::Frame::decode(bytes)
                .map(ClientRequest::ToolsTemplatesDelete)
                .ok(),
            33 => tools::templates::tag::client::request::Frame::decode(bytes)
                .map(ClientRequest::ToolsTemplatesTag)
                .ok(),
            34 => tools::templates::untag::client::request::Frame::decode(bytes)
                .map(ClientRequest::ToolsTemplatesUntag)
                .ok(),
            35 => resources::upload::client::request::Frame::decode(bytes)
                .map(ClientRequest::ResourcesUpload)
                .ok(),
            36 => resources::list::client::request::Frame::decode(bytes)
                .map(ClientRequest::ResourcesList)
                .ok(),
            37 => resources::delete::client::request::Frame::decode(bytes)
                .map(ClientRequest::ResourcesDelete)
                .ok(),
            38 => providers::outgoing::add::client::request::Frame::decode(bytes)
                .map(ClientRequest::ProvidersOutgoingAdd)
                .ok(),
            39 => providers::outgoing::get::client::request::Frame::decode(bytes)
                .map(ClientRequest::ProvidersOutgoingGet)
                .ok(),
            40 => providers::outgoing::list::client::request::Frame::decode(bytes)
                .map(ClientRequest::ProvidersOutgoingList)
                .ok(),
            41 => providers::outgoing::delete::client::request::Frame::decode(bytes)
                .map(ClientRequest::ProvidersOutgoingDelete)
                .ok(),
            42 => providers::outgoing::edit::client::request::Frame::decode(bytes)
                .map(ClientRequest::ProvidersOutgoingEdit)
                .ok(),
            43 => providers::incoming::add::client::request::Frame::decode(bytes)
                .map(ClientRequest::ProvidersIncomingAdd)
                .ok(),
            44 => providers::incoming::get::client::request::Frame::decode(bytes)
                .map(ClientRequest::ProvidersIncomingGet)
                .ok(),
            45 => providers::incoming::list::client::request::Frame::decode(bytes)
                .map(ClientRequest::ProvidersIncomingList)
                .ok(),
            46 => providers::incoming::delete::client::request::Frame::decode(bytes)
                .map(ClientRequest::ProvidersIncomingDelete)
                .ok(),
            47 => providers::incoming::edit::client::request::Frame::decode(bytes)
                .map(ClientRequest::ProvidersIncomingEdit)
                .ok(),
            48 => accounts::create::client::request::Frame::decode(bytes)
                .map(ClientRequest::AccountsCreate)
                .ok(),
            49 => accounts::get::client::request::Frame::decode(bytes)
                .map(ClientRequest::AccountsGet)
                .ok(),
            50 => accounts::list::client::request::Frame::decode(bytes)
                .map(ClientRequest::AccountsList)
                .ok(),
            51 => accounts::delete::client::request::Frame::decode(bytes)
                .map(ClientRequest::AccountsDelete)
                .ok(),
            52 => accounts::edit::client::request::Frame::decode(bytes)
                .map(ClientRequest::AccountsEdit)
                .ok(),
            53 => accounts::tag::client::request::Frame::decode(bytes)
                .map(ClientRequest::AccountsTag)
                .ok(),
            54 => accounts::untag::client::request::Frame::decode(bytes)
                .map(ClientRequest::AccountsUntag)
                .ok(),
            55 => roles::create::client::request::Frame::decode(bytes)
                .map(ClientRequest::RolesCreate)
                .ok(),
            56 => roles::get::client::request::Frame::decode(bytes)
                .map(ClientRequest::RolesGet)
                .ok(),
            57 => roles::list::client::request::Frame::decode(bytes)
                .map(ClientRequest::RolesList)
                .ok(),
            58 => roles::delete::client::request::Frame::decode(bytes)
                .map(ClientRequest::RolesDelete)
                .ok(),
            59 => roles::edit::client::request::Frame::decode(bytes)
                .map(ClientRequest::RolesEdit)
                .ok(),
            60 => roles::tag::client::request::Frame::decode(bytes)
                .map(ClientRequest::RolesTag)
                .ok(),
            61 => roles::untag::client::request::Frame::decode(bytes)
                .map(ClientRequest::RolesUntag)
                .ok(),
            62 => agents::download::client::request::Frame::decode(bytes)
                .map(ClientRequest::AgentsDownload)
                .ok(),
            63 => agents::upload::client::request::Frame::decode(bytes)
                .map(ClientRequest::AgentsUpload)
                .ok(),
            64 => agents::transfer::client::request::Frame::decode(bytes)
                .map(ClientRequest::AgentsTransfer)
                .ok(),
            65 => tools::download::client::request::Frame::decode(bytes)
                .map(ClientRequest::ToolsDownload)
                .ok(),
            66 => tools::upload::client::request::Frame::decode(bytes)
                .map(ClientRequest::ToolsUpload)
                .ok(),
            67 => tools::transfer::client::request::Frame::decode(bytes)
                .map(ClientRequest::ToolsTransfer)
                .ok(),
            68 => resources::download::client::request::Frame::decode(bytes)
                .map(ClientRequest::ResourcesDownload)
                .ok(),
            69 => resources::transfer::client::request::Frame::decode(bytes)
                .map(ClientRequest::ResourcesTransfer)
                .ok(),
            70 => volumes::create::client::request::Frame::decode(bytes)
                .map(ClientRequest::VolumesCreate)
                .ok(),
            71 => volumes::get::client::request::Frame::decode(bytes)
                .map(ClientRequest::VolumesGet)
                .ok(),
            72 => volumes::list::client::request::Frame::decode(bytes)
                .map(ClientRequest::VolumesList)
                .ok(),
            73 => volumes::delete::client::request::Frame::decode(bytes)
                .map(ClientRequest::VolumesDelete)
                .ok(),
            74 => volumes::edit::client::request::Frame::decode(bytes)
                .map(ClientRequest::VolumesEdit)
                .ok(),
            75 => volumes::stat::client::request::Frame::decode(bytes)
                .map(ClientRequest::VolumesStat)
                .ok(),
            76 => volumes::download::client::request::Frame::decode(bytes)
                .map(ClientRequest::VolumesDownload)
                .ok(),
            77 => volumes::upload::client::request::Frame::decode(bytes)
                .map(ClientRequest::VolumesUpload)
                .ok(),
            78 => volumes::transfer::client::request::Frame::decode(bytes)
                .map(ClientRequest::VolumesTransfer)
                .ok(),
            79 => agents::filetree::client::request::Frame::decode(bytes)
                .map(ClientRequest::AgentsFiletree)
                .ok(),
            80 => tools::filetree::client::request::Frame::decode(bytes)
                .map(ClientRequest::ToolsFiletree)
                .ok(),
            81 => resources::filetree::client::request::Frame::decode(bytes)
                .map(ClientRequest::ResourcesFiletree)
                .ok(),
            82 => volumes::filetree::client::request::Frame::decode(bytes)
                .map(ClientRequest::VolumesFiletree)
                .ok(),
            83 => resources::get::client::request::Frame::decode(bytes)
                .map(ClientRequest::ResourcesGet)
                .ok(),
            84 => resources::tag::client::request::Frame::decode(bytes)
                .map(ClientRequest::ResourcesTag)
                .ok(),
            85 => resources::untag::client::request::Frame::decode(bytes)
                .map(ClientRequest::ResourcesUntag)
                .ok(),
            _ => None,
        };
        Ok(request.unwrap_or(ClientRequest::Invalid(bytes)))
    }
}

impl fmt::Display for ClientRequest<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ClientRequest::AgentsCreate(_) => f.write_str("agents create"),
            ClientRequest::AgentsGet(_) => f.write_str("agents get"),
            ClientRequest::AgentsDelete(_) => f.write_str("agents delete"),
            ClientRequest::AgentsMessage(_) => f.write_str("agents message"),
            ClientRequest::AgentsLogs(_) => f.write_str("agents logs"),
            ClientRequest::AgentsList(_) => f.write_str("agents list"),
            ClientRequest::AgentsEdit(_) => f.write_str("agents edit"),
            ClientRequest::AgentsTag(_) => f.write_str("agents tag"),
            ClientRequest::AgentsUntag(_) => f.write_str("agents untag"),
            ClientRequest::AgentsTemplatesCreate(_) => f.write_str("agents templates create"),
            ClientRequest::AgentsTemplatesGet(_) => f.write_str("agents templates get"),
            ClientRequest::AgentsTemplatesList(_) => f.write_str("agents templates list"),
            ClientRequest::AgentsTemplatesDelete(_) => f.write_str("agents templates delete"),
            ClientRequest::AgentsTemplatesTag(_) => f.write_str("agents templates tag"),
            ClientRequest::AgentsTemplatesUntag(_) => f.write_str("agents templates untag"),
            ClientRequest::ToolsCreate(_) => f.write_str("tools create"),
            ClientRequest::ToolsGet(_) => f.write_str("tools get"),
            ClientRequest::ToolsEdit(_) => f.write_str("tools edit"),
            ClientRequest::ToolsConnect(_) => f.write_str("tools connect"),
            ClientRequest::ToolsListFor(_) => f.write_str("tools list_for"),
            ClientRequest::ToolsAttach(_) => f.write_str("tools attach"),
            ClientRequest::ToolsDetach(_) => f.write_str("tools detach"),
            ClientRequest::ToolsDelete(_) => f.write_str("tools delete"),
            ClientRequest::ToolsList(_) => f.write_str("tools list"),
            ClientRequest::ToolsTag(_) => f.write_str("tools tag"),
            ClientRequest::ToolsUntag(_) => f.write_str("tools untag"),
            ClientRequest::ToolsRoutesSet(_) => f.write_str("tools routes set"),
            ClientRequest::ToolsRoutesDelete(_) => f.write_str("tools routes delete"),
            ClientRequest::ToolsRoutesList(_) => f.write_str("tools routes list"),
            ClientRequest::ToolsTemplatesCreate(_) => f.write_str("tools templates create"),
            ClientRequest::ToolsTemplatesGet(_) => f.write_str("tools templates get"),
            ClientRequest::ToolsTemplatesList(_) => f.write_str("tools templates list"),
            ClientRequest::ToolsTemplatesDelete(_) => f.write_str("tools templates delete"),
            ClientRequest::ToolsTemplatesTag(_) => f.write_str("tools templates tag"),
            ClientRequest::ToolsTemplatesUntag(_) => f.write_str("tools templates untag"),
            ClientRequest::ResourcesUpload(_) => f.write_str("resources upload"),
            ClientRequest::ResourcesList(_) => f.write_str("resources list"),
            ClientRequest::ResourcesDelete(_) => f.write_str("resources delete"),
            ClientRequest::ProvidersOutgoingAdd(_) => f.write_str("providers outgoing add"),
            ClientRequest::ProvidersOutgoingGet(_) => f.write_str("providers outgoing get"),
            ClientRequest::ProvidersOutgoingList(_) => f.write_str("providers outgoing list"),
            ClientRequest::ProvidersOutgoingDelete(_) => f.write_str("providers outgoing delete"),
            ClientRequest::ProvidersOutgoingEdit(_) => f.write_str("providers outgoing edit"),
            ClientRequest::ProvidersIncomingAdd(_) => f.write_str("providers incoming add"),
            ClientRequest::ProvidersIncomingGet(_) => f.write_str("providers incoming get"),
            ClientRequest::ProvidersIncomingList(_) => f.write_str("providers incoming list"),
            ClientRequest::ProvidersIncomingDelete(_) => f.write_str("providers incoming delete"),
            ClientRequest::ProvidersIncomingEdit(_) => f.write_str("providers incoming edit"),
            ClientRequest::AccountsCreate(_) => f.write_str("accounts create"),
            ClientRequest::AccountsGet(_) => f.write_str("accounts get"),
            ClientRequest::AccountsList(_) => f.write_str("accounts list"),
            ClientRequest::AccountsDelete(_) => f.write_str("accounts delete"),
            ClientRequest::AccountsEdit(_) => f.write_str("accounts edit"),
            ClientRequest::AccountsTag(_) => f.write_str("accounts tag"),
            ClientRequest::AccountsUntag(_) => f.write_str("accounts untag"),
            ClientRequest::RolesCreate(_) => f.write_str("roles create"),
            ClientRequest::RolesGet(_) => f.write_str("roles get"),
            ClientRequest::RolesList(_) => f.write_str("roles list"),
            ClientRequest::RolesDelete(_) => f.write_str("roles delete"),
            ClientRequest::RolesEdit(_) => f.write_str("roles edit"),
            ClientRequest::RolesTag(_) => f.write_str("roles tag"),
            ClientRequest::RolesUntag(_) => f.write_str("roles untag"),
            ClientRequest::AgentsDownload(_) => f.write_str("agents download"),
            ClientRequest::AgentsUpload(_) => f.write_str("agents upload"),
            ClientRequest::AgentsTransfer(_) => f.write_str("agents transfer"),
            ClientRequest::ToolsDownload(_) => f.write_str("tools download"),
            ClientRequest::ToolsUpload(_) => f.write_str("tools upload"),
            ClientRequest::ToolsTransfer(_) => f.write_str("tools transfer"),
            ClientRequest::ResourcesDownload(_) => f.write_str("resources download"),
            ClientRequest::ResourcesTransfer(_) => f.write_str("resources transfer"),
            ClientRequest::VolumesCreate(_) => f.write_str("volumes create"),
            ClientRequest::VolumesGet(_) => f.write_str("volumes get"),
            ClientRequest::VolumesList(_) => f.write_str("volumes list"),
            ClientRequest::VolumesDelete(_) => f.write_str("volumes delete"),
            ClientRequest::VolumesEdit(_) => f.write_str("volumes edit"),
            ClientRequest::VolumesStat(_) => f.write_str("volumes stat"),
            ClientRequest::VolumesDownload(_) => f.write_str("volumes download"),
            ClientRequest::VolumesUpload(_) => f.write_str("volumes upload"),
            ClientRequest::VolumesTransfer(_) => f.write_str("volumes transfer"),
            ClientRequest::AgentsFiletree(_) => f.write_str("agents filetree"),
            ClientRequest::ToolsFiletree(_) => f.write_str("tools filetree"),
            ClientRequest::ResourcesFiletree(_) => f.write_str("resources filetree"),
            ClientRequest::VolumesFiletree(_) => f.write_str("volumes filetree"),
            ClientRequest::ResourcesGet(_) => f.write_str("resources get"),
            ClientRequest::ResourcesTag(_) => f.write_str("resources tag"),
            ClientRequest::ResourcesUntag(_) => f.write_str("resources untag"),
            ClientRequest::Invalid(_) => f.write_str("an invalid request"),
        }
    }
}
