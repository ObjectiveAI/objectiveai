//! Who may see a tool from its provider, and who may join it: the
//! tool's admissions.

use std::net::IpAddr;

use diverge_sdk::daemon::endpoints::tools::Admits;
use diverge_sdk::provider::client::ConnectionAuthorizer;
use diverge_sdk::shared::containers::authorize::request::{AuthorizeConnect, AuthorizeList};
use diverge_sdk::shared::containers::authorize::response::Frame;

use super::Answerer;
use crate::containers::{Key, ToolKey};
use crate::judge::key;
use crate::store::tools::admissions;

/// Default deny. A list is yes when an admission on the tool names
/// the lister's identity, its address if it names one, and admits a
/// list; a connect is yes when the connector's authorization is an
/// admission's key, the admission is this tool's, its address fits,
/// and it admits a connect. An agent container, listed to nobody and
/// taking no connector, is asked neither and would say no; a
/// dependency tool, nobody's record, has no admissions and says no.
impl ConnectionAuthorizer for Answerer {
    async fn authorize_list(&self, request: &AuthorizeList) -> Frame {
        let Key::Tool(ToolKey::Record(tool)) = self.key else {
            return Frame::Denied;
        };
        let Ok(mut conn) = self.daemon.store.acquire().await else {
            return Frame::Denied;
        };
        let Ok(admissions) = admissions::of_tool(&mut conn, tool).await else {
            return Frame::Denied;
        };
        let admitted = admissions.iter().any(|record| {
            record.admission.identity == request.identity
                && fits(record.admission.address, request.address)
                && matches!(record.admission.admits, Admits::List | Admits::Both)
        });
        if admitted { Frame::Authorized } else { Frame::Denied }
    }

    async fn authorize_connect(&self, request: &AuthorizeConnect) -> Frame {
        let Key::Tool(ToolKey::Record(tool)) = self.key else {
            return Frame::Denied;
        };
        if request.authorization.is_empty() {
            return Frame::Denied;
        }
        let Ok(mut conn) = self.daemon.store.acquire().await else {
            return Frame::Denied;
        };
        let Ok(Some(record)) = admissions::by_key_hash(&mut conn, &key::hash(&request.authorization)).await else {
            return Frame::Denied;
        };
        let admitted = record.tool == tool
            && fits(record.admission.address, request.address)
            && matches!(record.admission.admits, Admits::Connect | Admits::Both);
        if admitted { Frame::Authorized } else { Frame::Denied }
    }
}

/// Whether the address the admission names, if any, is the one seen.
fn fits(named: Option<IpAddr>, seen: IpAddr) -> bool {
    named.is_none_or(|named| named.to_canonical() == seen.to_canonical())
}
