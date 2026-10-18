//! An outgoing provider's address, as the URL the daemon dials.

/// The URL for `address`: the address itself when it carries a
/// scheme — `wss://provider.example/` — and otherwise `ws://` in front
/// of it and `/` behind, as the provider's own dial makes one of a
/// `host:port`. The address is never changed for naming: the
/// provider's identity is the string as given.
pub fn url(address: &str) -> String {
    if address.contains("://") {
        address.to_string()
    } else {
        format!("ws://{address}/")
    }
}
