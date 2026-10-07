//! Who a credential is.

use std::net::IpAddr;

/// The account a credential names, judged as the provider judges a
/// peer that dials in: by the keys the daemon holds, and by the
/// address the key is for when it is for one.
///
/// There is no account yet, and no key, so there is nothing to judge
/// by and every credential is refused: `None`, and the connection is
/// closed without a word. The root key the first client presents, and
/// the accounts the root makes, come with the store that will hold
/// them.
pub async fn judge(credential: &str, address: IpAddr) -> Option<String> {
    let _ = (credential, address);
    None
}
