//! Every request a client can open a scope with.

use std::convert::Infallible;
use std::fmt;

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

use super::{accounts, agents, postgres, providers, roles, tools, volumes};

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
    /// Tag `5`. List the caller's agents, narrowed, and keep the list.
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
    /// Tag `11`. List the caller's templates, narrowed, and keep the list.
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
    /// Tag `19`. Attach a tool to an agent.
    ToolsAttach(tools::attach::client::request::Frame),
    /// Tag `20`. Detach a tool from an agent.
    ToolsDetach(tools::detach::client::request::Frame),
    /// Tag `21`. Delete a tool.
    ToolsDelete(tools::delete::client::request::Frame),
    /// Tag `22`. List the caller's tools, narrowed, and keep the list.
    ToolsList(tools::list::client::request::Frame),
    /// Tag `23`. Put tags on a tool.
    ToolsTag(tools::tag::client::request::Frame),
    /// Tag `24`. Take tags off a tool.
    ToolsUntag(tools::untag::client::request::Frame),
    /// Tag `25`. Make a tool template.
    ToolsTemplatesCreate(tools::templates::create::client::request::Frame),
    /// Tag `26`. Get one tool template by id.
    ToolsTemplatesGet(tools::templates::get::client::request::Frame),
    /// Tag `27`. List the caller's tool templates, narrowed, and keep the list.
    ToolsTemplatesList(tools::templates::list::client::request::Frame),
    /// Tag `28`. Delete a tool template by id.
    ToolsTemplatesDelete(tools::templates::delete::client::request::Frame),
    /// Tag `29`. Put tags on a tool template.
    ToolsTemplatesTag(tools::templates::tag::client::request::Frame),
    /// Tag `30`. Take tags off a tool template.
    ToolsTemplatesUntag(tools::templates::untag::client::request::Frame),
    /// Tag `31`. Add a provider to dial.
    ProvidersOutgoingAdd(providers::outgoing::add::client::request::Frame),
    /// Tag `32`. Get one outgoing provider.
    ProvidersOutgoingGet(providers::outgoing::get::client::request::Frame),
    /// Tag `33`. List the caller's outgoing providers, narrowed.
    ProvidersOutgoingList(providers::outgoing::list::client::request::Frame),
    /// Tag `34`. Forget an outgoing provider.
    ProvidersOutgoingDelete(providers::outgoing::delete::client::request::Frame),
    /// Tag `35`. Replace an outgoing provider's mode.
    ProvidersOutgoingEdit(providers::outgoing::edit::client::request::Frame),
    /// Tag `36`. Put tags on an outgoing provider.
    ProvidersOutgoingTag(providers::outgoing::tag::client::request::Frame),
    /// Tag `37`. Take tags off an outgoing provider.
    ProvidersOutgoingUntag(providers::outgoing::untag::client::request::Frame),
    /// Tag `38`. Add a credential of incoming providers.
    ProvidersIncomingAdd(providers::incoming::add::client::request::Frame),
    /// Tag `39`. Get one credential.
    ProvidersIncomingGet(providers::incoming::get::client::request::Frame),
    /// Tag `40`. List the credentials, narrowed.
    ProvidersIncomingList(providers::incoming::list::client::request::Frame),
    /// Tag `41`. Take a credential out.
    ProvidersIncomingDelete(providers::incoming::delete::client::request::Frame),
    /// Tag `42`. Replace a credential.
    ProvidersIncomingEdit(providers::incoming::edit::client::request::Frame),
    /// Tag `43`. Put tags on a credential.
    ProvidersIncomingTag(providers::incoming::tag::client::request::Frame),
    /// Tag `44`. Take tags off a credential.
    ProvidersIncomingUntag(providers::incoming::untag::client::request::Frame),
    /// Tag `45`. Create an account.
    AccountsCreate(accounts::create::client::request::Frame),
    /// Tag `46`. Get one account.
    AccountsGet(accounts::get::client::request::Frame),
    /// Tag `47`. List the accounts, narrowed, and keep the list.
    AccountsList(accounts::list::client::request::Frame),
    /// Tag `48`. Delete an account.
    AccountsDelete(accounts::delete::client::request::Frame),
    /// Tag `49`. Change an account.
    AccountsEdit(accounts::edit::client::request::Frame),
    /// Tag `50`. Put tags on an account.
    AccountsTag(accounts::tag::client::request::Frame),
    /// Tag `51`. Take tags off an account.
    AccountsUntag(accounts::untag::client::request::Frame),
    /// Tag `52`. Create a role.
    RolesCreate(roles::create::client::request::Frame),
    /// Tag `53`. Get one role.
    RolesGet(roles::get::client::request::Frame),
    /// Tag `54`. List the roles, narrowed, and keep the list.
    RolesList(roles::list::client::request::Frame),
    /// Tag `55`. Delete a role.
    RolesDelete(roles::delete::client::request::Frame),
    /// Tag `56`. Change a role.
    RolesEdit(roles::edit::client::request::Frame),
    /// Tag `57`. Put tags on a role.
    RolesTag(roles::tag::client::request::Frame),
    /// Tag `58`. Take tags off a role.
    RolesUntag(roles::untag::client::request::Frame),
    /// Tag `59`. Send the client files out of an agent's container.
    AgentsDownload(agents::download::client::request::Frame),
    /// Tag `60`. Put files into an agent's container.
    AgentsUpload(agents::upload::client::request::Frame),
    /// Tag `61`. Copy files out of an agent's container elsewhere.
    AgentsTransfer(agents::transfer::client::request::Frame),
    /// Tag `62`. Send the client files out of a tool's container.
    ToolsDownload(tools::download::client::request::Frame),
    /// Tag `63`. Put files into a tool's container.
    ToolsUpload(tools::upload::client::request::Frame),
    /// Tag `64`. Copy files out of a tool's container elsewhere.
    ToolsTransfer(tools::transfer::client::request::Frame),
    /// Tag `65`. Create a volume on a provider.
    VolumesCreate(volumes::create::client::request::Frame),
    /// Tag `66`. Get one volume.
    VolumesGet(volumes::get::client::request::Frame),
    /// Tag `67`. List the volumes, narrowed.
    VolumesList(volumes::list::client::request::Frame),
    /// Tag `68`. Delete a volume.
    VolumesDelete(volumes::delete::client::request::Frame),
    /// Tag `69`. Change a volume's size or mode.
    VolumesEdit(volumes::edit::client::request::Frame),
    /// Tag `70`. Put tags on a volume.
    VolumesTag(volumes::tag::client::request::Frame),
    /// Tag `71`. Take tags off a volume.
    VolumesUntag(volumes::untag::client::request::Frame),
    /// Tag `72`. Walk a volume for its use and its hash.
    VolumesStat(volumes::stat::client::request::Frame),
    /// Tag `73`. Send the client files out of a volume.
    VolumesDownload(volumes::download::client::request::Frame),
    /// Tag `74`. Put files into a volume.
    VolumesUpload(volumes::upload::client::request::Frame),
    /// Tag `75`. Copy files out of a volume elsewhere.
    VolumesTransfer(volumes::transfer::client::request::Frame),
    /// Tag `76`. Watch an agent's container whole.
    AgentsFiletree(agents::filetree::client::request::Frame),
    /// Tag `77`. Watch a tool's container whole.
    ToolsFiletree(tools::filetree::client::request::Frame),
    /// Tag `78`. See a volume's tree, once.
    VolumesFiletree(volumes::filetree::client::request::Frame),
    /// Tag `79`. Read which database the daemon serves.
    PostgresGet(postgres::get::client::request::Frame),
    /// Tag `80`. List the container connections open through the database.
    PostgresList(postgres::list::client::request::Frame),
    /// Tag `81`. Admit a lister or a connector to a tool.
    ToolsAdmit(tools::admit::client::request::Frame),
    /// Tag `82`. Take an admission off a tool.
    ToolsUnadmit(tools::unadmit::client::request::Frame),
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
            ClientRequest::ToolsAttach(frame) => frame.encode(out),
            ClientRequest::ToolsDetach(frame) => frame.encode(out),
            ClientRequest::ToolsDelete(frame) => frame.encode(out),
            ClientRequest::ToolsList(frame) => frame.encode(out),
            ClientRequest::ToolsTag(frame) => frame.encode(out),
            ClientRequest::ToolsUntag(frame) => frame.encode(out),
            ClientRequest::ToolsTemplatesCreate(frame) => frame.encode(out),
            ClientRequest::ToolsTemplatesGet(frame) => frame.encode(out),
            ClientRequest::ToolsTemplatesList(frame) => frame.encode(out),
            ClientRequest::ToolsTemplatesDelete(frame) => frame.encode(out),
            ClientRequest::ToolsTemplatesTag(frame) => frame.encode(out),
            ClientRequest::ToolsTemplatesUntag(frame) => frame.encode(out),
            ClientRequest::ProvidersOutgoingAdd(frame) => frame.encode(out),
            ClientRequest::ProvidersOutgoingGet(frame) => frame.encode(out),
            ClientRequest::ProvidersOutgoingList(frame) => frame.encode(out),
            ClientRequest::ProvidersOutgoingDelete(frame) => frame.encode(out),
            ClientRequest::ProvidersOutgoingEdit(frame) => frame.encode(out),
            ClientRequest::ProvidersOutgoingTag(frame) => frame.encode(out),
            ClientRequest::ProvidersOutgoingUntag(frame) => frame.encode(out),
            ClientRequest::ProvidersIncomingAdd(frame) => frame.encode(out),
            ClientRequest::ProvidersIncomingGet(frame) => frame.encode(out),
            ClientRequest::ProvidersIncomingList(frame) => frame.encode(out),
            ClientRequest::ProvidersIncomingDelete(frame) => frame.encode(out),
            ClientRequest::ProvidersIncomingEdit(frame) => frame.encode(out),
            ClientRequest::ProvidersIncomingTag(frame) => frame.encode(out),
            ClientRequest::ProvidersIncomingUntag(frame) => frame.encode(out),
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
            ClientRequest::VolumesCreate(frame) => frame.encode(out),
            ClientRequest::VolumesGet(frame) => frame.encode(out),
            ClientRequest::VolumesList(frame) => frame.encode(out),
            ClientRequest::VolumesDelete(frame) => frame.encode(out),
            ClientRequest::VolumesEdit(frame) => frame.encode(out),
            ClientRequest::VolumesTag(frame) => frame.encode(out),
            ClientRequest::VolumesUntag(frame) => frame.encode(out),
            ClientRequest::VolumesStat(frame) => frame.encode(out),
            ClientRequest::VolumesDownload(frame) => frame.encode(out),
            ClientRequest::VolumesUpload(frame) => frame.encode(out),
            ClientRequest::VolumesTransfer(frame) => frame.encode(out),
            ClientRequest::AgentsFiletree(frame) => frame.encode(out),
            ClientRequest::ToolsFiletree(frame) => frame.encode(out),
            ClientRequest::VolumesFiletree(frame) => frame.encode(out),
            ClientRequest::PostgresGet(frame) => frame.encode(out),
            ClientRequest::PostgresList(frame) => frame.encode(out),
            ClientRequest::ToolsAdmit(frame) => frame.encode(out),
            ClientRequest::ToolsUnadmit(frame) => frame.encode(out),
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
            19 => tools::attach::client::request::Frame::decode(bytes)
                .map(ClientRequest::ToolsAttach)
                .ok(),
            20 => tools::detach::client::request::Frame::decode(bytes)
                .map(ClientRequest::ToolsDetach)
                .ok(),
            21 => tools::delete::client::request::Frame::decode(bytes)
                .map(ClientRequest::ToolsDelete)
                .ok(),
            22 => tools::list::client::request::Frame::decode(bytes)
                .map(ClientRequest::ToolsList)
                .ok(),
            23 => tools::tag::client::request::Frame::decode(bytes)
                .map(ClientRequest::ToolsTag)
                .ok(),
            24 => tools::untag::client::request::Frame::decode(bytes)
                .map(ClientRequest::ToolsUntag)
                .ok(),
            25 => tools::templates::create::client::request::Frame::decode(bytes)
                .map(ClientRequest::ToolsTemplatesCreate)
                .ok(),
            26 => tools::templates::get::client::request::Frame::decode(bytes)
                .map(ClientRequest::ToolsTemplatesGet)
                .ok(),
            27 => tools::templates::list::client::request::Frame::decode(bytes)
                .map(ClientRequest::ToolsTemplatesList)
                .ok(),
            28 => tools::templates::delete::client::request::Frame::decode(bytes)
                .map(ClientRequest::ToolsTemplatesDelete)
                .ok(),
            29 => tools::templates::tag::client::request::Frame::decode(bytes)
                .map(ClientRequest::ToolsTemplatesTag)
                .ok(),
            30 => tools::templates::untag::client::request::Frame::decode(bytes)
                .map(ClientRequest::ToolsTemplatesUntag)
                .ok(),
            31 => providers::outgoing::add::client::request::Frame::decode(bytes)
                .map(ClientRequest::ProvidersOutgoingAdd)
                .ok(),
            32 => providers::outgoing::get::client::request::Frame::decode(bytes)
                .map(ClientRequest::ProvidersOutgoingGet)
                .ok(),
            33 => providers::outgoing::list::client::request::Frame::decode(bytes)
                .map(ClientRequest::ProvidersOutgoingList)
                .ok(),
            34 => providers::outgoing::delete::client::request::Frame::decode(bytes)
                .map(ClientRequest::ProvidersOutgoingDelete)
                .ok(),
            35 => providers::outgoing::edit::client::request::Frame::decode(bytes)
                .map(ClientRequest::ProvidersOutgoingEdit)
                .ok(),
            36 => providers::outgoing::tag::client::request::Frame::decode(bytes)
                .map(ClientRequest::ProvidersOutgoingTag)
                .ok(),
            37 => providers::outgoing::untag::client::request::Frame::decode(bytes)
                .map(ClientRequest::ProvidersOutgoingUntag)
                .ok(),
            38 => providers::incoming::add::client::request::Frame::decode(bytes)
                .map(ClientRequest::ProvidersIncomingAdd)
                .ok(),
            39 => providers::incoming::get::client::request::Frame::decode(bytes)
                .map(ClientRequest::ProvidersIncomingGet)
                .ok(),
            40 => providers::incoming::list::client::request::Frame::decode(bytes)
                .map(ClientRequest::ProvidersIncomingList)
                .ok(),
            41 => providers::incoming::delete::client::request::Frame::decode(bytes)
                .map(ClientRequest::ProvidersIncomingDelete)
                .ok(),
            42 => providers::incoming::edit::client::request::Frame::decode(bytes)
                .map(ClientRequest::ProvidersIncomingEdit)
                .ok(),
            43 => providers::incoming::tag::client::request::Frame::decode(bytes)
                .map(ClientRequest::ProvidersIncomingTag)
                .ok(),
            44 => providers::incoming::untag::client::request::Frame::decode(bytes)
                .map(ClientRequest::ProvidersIncomingUntag)
                .ok(),
            45 => accounts::create::client::request::Frame::decode(bytes)
                .map(ClientRequest::AccountsCreate)
                .ok(),
            46 => accounts::get::client::request::Frame::decode(bytes)
                .map(ClientRequest::AccountsGet)
                .ok(),
            47 => accounts::list::client::request::Frame::decode(bytes)
                .map(ClientRequest::AccountsList)
                .ok(),
            48 => accounts::delete::client::request::Frame::decode(bytes)
                .map(ClientRequest::AccountsDelete)
                .ok(),
            49 => accounts::edit::client::request::Frame::decode(bytes)
                .map(ClientRequest::AccountsEdit)
                .ok(),
            50 => accounts::tag::client::request::Frame::decode(bytes)
                .map(ClientRequest::AccountsTag)
                .ok(),
            51 => accounts::untag::client::request::Frame::decode(bytes)
                .map(ClientRequest::AccountsUntag)
                .ok(),
            52 => roles::create::client::request::Frame::decode(bytes)
                .map(ClientRequest::RolesCreate)
                .ok(),
            53 => roles::get::client::request::Frame::decode(bytes)
                .map(ClientRequest::RolesGet)
                .ok(),
            54 => roles::list::client::request::Frame::decode(bytes)
                .map(ClientRequest::RolesList)
                .ok(),
            55 => roles::delete::client::request::Frame::decode(bytes)
                .map(ClientRequest::RolesDelete)
                .ok(),
            56 => roles::edit::client::request::Frame::decode(bytes)
                .map(ClientRequest::RolesEdit)
                .ok(),
            57 => roles::tag::client::request::Frame::decode(bytes)
                .map(ClientRequest::RolesTag)
                .ok(),
            58 => roles::untag::client::request::Frame::decode(bytes)
                .map(ClientRequest::RolesUntag)
                .ok(),
            59 => agents::download::client::request::Frame::decode(bytes)
                .map(ClientRequest::AgentsDownload)
                .ok(),
            60 => agents::upload::client::request::Frame::decode(bytes)
                .map(ClientRequest::AgentsUpload)
                .ok(),
            61 => agents::transfer::client::request::Frame::decode(bytes)
                .map(ClientRequest::AgentsTransfer)
                .ok(),
            62 => tools::download::client::request::Frame::decode(bytes)
                .map(ClientRequest::ToolsDownload)
                .ok(),
            63 => tools::upload::client::request::Frame::decode(bytes)
                .map(ClientRequest::ToolsUpload)
                .ok(),
            64 => tools::transfer::client::request::Frame::decode(bytes)
                .map(ClientRequest::ToolsTransfer)
                .ok(),
            65 => volumes::create::client::request::Frame::decode(bytes)
                .map(ClientRequest::VolumesCreate)
                .ok(),
            66 => volumes::get::client::request::Frame::decode(bytes)
                .map(ClientRequest::VolumesGet)
                .ok(),
            67 => volumes::list::client::request::Frame::decode(bytes)
                .map(ClientRequest::VolumesList)
                .ok(),
            68 => volumes::delete::client::request::Frame::decode(bytes)
                .map(ClientRequest::VolumesDelete)
                .ok(),
            69 => volumes::edit::client::request::Frame::decode(bytes)
                .map(ClientRequest::VolumesEdit)
                .ok(),
            70 => volumes::tag::client::request::Frame::decode(bytes)
                .map(ClientRequest::VolumesTag)
                .ok(),
            71 => volumes::untag::client::request::Frame::decode(bytes)
                .map(ClientRequest::VolumesUntag)
                .ok(),
            72 => volumes::stat::client::request::Frame::decode(bytes)
                .map(ClientRequest::VolumesStat)
                .ok(),
            73 => volumes::download::client::request::Frame::decode(bytes)
                .map(ClientRequest::VolumesDownload)
                .ok(),
            74 => volumes::upload::client::request::Frame::decode(bytes)
                .map(ClientRequest::VolumesUpload)
                .ok(),
            75 => volumes::transfer::client::request::Frame::decode(bytes)
                .map(ClientRequest::VolumesTransfer)
                .ok(),
            76 => agents::filetree::client::request::Frame::decode(bytes)
                .map(ClientRequest::AgentsFiletree)
                .ok(),
            77 => tools::filetree::client::request::Frame::decode(bytes)
                .map(ClientRequest::ToolsFiletree)
                .ok(),
            78 => volumes::filetree::client::request::Frame::decode(bytes)
                .map(ClientRequest::VolumesFiletree)
                .ok(),
            79 => postgres::get::client::request::Frame::decode(bytes)
                .map(ClientRequest::PostgresGet)
                .ok(),
            80 => postgres::list::client::request::Frame::decode(bytes)
                .map(ClientRequest::PostgresList)
                .ok(),
            81 => tools::admit::client::request::Frame::decode(bytes)
                .map(ClientRequest::ToolsAdmit)
                .ok(),
            82 => tools::unadmit::client::request::Frame::decode(bytes)
                .map(ClientRequest::ToolsUnadmit)
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
            ClientRequest::ToolsAttach(_) => f.write_str("tools attach"),
            ClientRequest::ToolsDetach(_) => f.write_str("tools detach"),
            ClientRequest::ToolsDelete(_) => f.write_str("tools delete"),
            ClientRequest::ToolsList(_) => f.write_str("tools list"),
            ClientRequest::ToolsTag(_) => f.write_str("tools tag"),
            ClientRequest::ToolsUntag(_) => f.write_str("tools untag"),
            ClientRequest::ToolsTemplatesCreate(_) => f.write_str("tools templates create"),
            ClientRequest::ToolsTemplatesGet(_) => f.write_str("tools templates get"),
            ClientRequest::ToolsTemplatesList(_) => f.write_str("tools templates list"),
            ClientRequest::ToolsTemplatesDelete(_) => f.write_str("tools templates delete"),
            ClientRequest::ToolsTemplatesTag(_) => f.write_str("tools templates tag"),
            ClientRequest::ToolsTemplatesUntag(_) => f.write_str("tools templates untag"),
            ClientRequest::ProvidersOutgoingAdd(_) => f.write_str("providers outgoing add"),
            ClientRequest::ProvidersOutgoingGet(_) => f.write_str("providers outgoing get"),
            ClientRequest::ProvidersOutgoingList(_) => f.write_str("providers outgoing list"),
            ClientRequest::ProvidersOutgoingDelete(_) => f.write_str("providers outgoing delete"),
            ClientRequest::ProvidersOutgoingEdit(_) => f.write_str("providers outgoing edit"),
            ClientRequest::ProvidersOutgoingTag(_) => f.write_str("providers outgoing tag"),
            ClientRequest::ProvidersOutgoingUntag(_) => f.write_str("providers outgoing untag"),
            ClientRequest::ProvidersIncomingAdd(_) => f.write_str("providers incoming add"),
            ClientRequest::ProvidersIncomingGet(_) => f.write_str("providers incoming get"),
            ClientRequest::ProvidersIncomingList(_) => f.write_str("providers incoming list"),
            ClientRequest::ProvidersIncomingDelete(_) => f.write_str("providers incoming delete"),
            ClientRequest::ProvidersIncomingEdit(_) => f.write_str("providers incoming edit"),
            ClientRequest::ProvidersIncomingTag(_) => f.write_str("providers incoming tag"),
            ClientRequest::ProvidersIncomingUntag(_) => f.write_str("providers incoming untag"),
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
            ClientRequest::VolumesCreate(_) => f.write_str("volumes create"),
            ClientRequest::VolumesGet(_) => f.write_str("volumes get"),
            ClientRequest::VolumesList(_) => f.write_str("volumes list"),
            ClientRequest::VolumesDelete(_) => f.write_str("volumes delete"),
            ClientRequest::VolumesEdit(_) => f.write_str("volumes edit"),
            ClientRequest::VolumesTag(_) => f.write_str("volumes tag"),
            ClientRequest::VolumesUntag(_) => f.write_str("volumes untag"),
            ClientRequest::VolumesStat(_) => f.write_str("volumes stat"),
            ClientRequest::VolumesDownload(_) => f.write_str("volumes download"),
            ClientRequest::VolumesUpload(_) => f.write_str("volumes upload"),
            ClientRequest::VolumesTransfer(_) => f.write_str("volumes transfer"),
            ClientRequest::AgentsFiletree(_) => f.write_str("agents filetree"),
            ClientRequest::ToolsFiletree(_) => f.write_str("tools filetree"),
            ClientRequest::VolumesFiletree(_) => f.write_str("volumes filetree"),
            ClientRequest::PostgresGet(_) => f.write_str("postgres get"),
            ClientRequest::PostgresList(_) => f.write_str("postgres list"),
            ClientRequest::ToolsAdmit(_) => f.write_str("tools admit"),
            ClientRequest::ToolsUnadmit(_) => f.write_str("tools unadmit"),
            ClientRequest::Invalid(_) => f.write_str("an invalid request"),
        }
    }
}
