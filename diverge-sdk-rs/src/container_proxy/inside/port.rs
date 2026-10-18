//! The port the program listens on, and the proxy dials.

/// The port the program listens on when the environment does not say
/// otherwise.
pub const DEFAULT_PORT: u16 = 8080;

/// The environment variable that names another port.
pub const PORT_VARIABLE: &str = "PORT";

/// The port the program listens on, and the proxy dials.
///
/// [`DEFAULT_PORT`], `8080`, unless the environment names another in
/// [`PORT_VARIABLE`] — `PORT`, the name every platform already uses —
/// and names a valid one. The program binds it; the proxy dials it;
/// both read the same environment, so the number is never written in
/// two places. The rules, in order:
///
/// 1. [`PORT_VARIABLE`] set, and its value a decimal number in
///    `1..=65535`: that number. Surrounding whitespace is not part of
///    a number, and is not tolerated.
/// 2. Otherwise — unset, empty, not a number, `0`, or out of range —
///    [`DEFAULT_PORT`].
///
/// Read every time it is called: the environment is the source, and
/// nothing here caches it.
pub fn port() -> u16 {
    std::env::var(PORT_VARIABLE)
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .filter(|port| *port != 0)
        .unwrap_or(DEFAULT_PORT)
}
