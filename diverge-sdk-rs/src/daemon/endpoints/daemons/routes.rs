//! Which routes a remote daemon's record may be reached by.

use serde::{Deserialize, Serialize};

use crate::daemon::endpoints::providers::Ip;
use crate::daemon::endpoints::providers::discover::server::response::Via;
use crate::shared::Identity;

/// The routes a record lets the daemon reach its remote by, out of the
/// [`Via`]s discovery holds for it: three filters, each optional, their
/// members beside the record's own. The rules:
///
/// 1. **Absent is no filter.** A filter not given, or given as an empty
///    list, lets every route through; a record with none reaches the
///    remote by any route. The two are one record: its
///    [`DaemonId`](super::DaemonId) hashes them alike.
/// 2. **Within a filter, any entry.** A route passes `providers` when
///    its provider is one of them, `ips` when its IP is in one of them
///    — see [`Ip::contains`] — and `addresses` when its dial target is
///    exactly one of them.
/// 3. **Across filters, all.** A route is let through only when it
///    passes every filter given.
/// 4. **Empty is absent.** A filter given as an empty list is no filter,
///    as rule 1 says: there is no filter that lets nothing through.
/// 5. **Addresses need a dial.** A route whose provider did not dial
///    the remote has no address, and passes no `addresses` filter.
/// 6. **Filters, not checks.** Nothing here is checked when the record
///    is added or edited: a provider named need not be connected, and
///    the filters are applied to discovery as it stands at each
///    connect.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
pub struct Routes {
    /// The providers it may be reached through, by identity. Absent, or
    /// empty, any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub providers: Option<Vec<Identity>>,
    /// The IPs, or blocks, its connection to that provider may be at.
    /// Absent, or empty, any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ips: Option<Vec<Ip>>,
    /// The dial targets that provider may have dialled it at — each a
    /// `host:port` or a URL, as the provider's configuration writes it.
    /// Absent, or empty, any, a route the provider did not dial among
    /// them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub addresses: Option<Vec<String>>,
}

impl Routes {
    /// Whether `via` is let through: rules 1 to 5.
    pub fn admits(&self, via: &Via) -> bool {
        // Rules 1 and 4: absent and empty alike are no filter.
        let provider = self.providers.as_ref().filter(|providers| !providers.is_empty()).is_none_or(|providers| providers.contains(&via.provider));
        let ip = self.ips.as_ref().filter(|ips| !ips.is_empty()).is_none_or(|ips| ips.iter().any(|allowed| allowed.contains(via.ip)));
        let address = self.addresses.as_ref().filter(|addresses| !addresses.is_empty()).is_none_or(|addresses| via.address.as_ref().is_some_and(|dialled| addresses.contains(dialled)));
        provider && ip && address
    }
}
