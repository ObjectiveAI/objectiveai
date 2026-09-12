//! The provider's configuration: where it lives, how it is found, and
//! what it says.
//!
//! Nothing is here yet. The rulings it will implement:
//!
//! - `--config <dir>`, else `DIVERGE_PROVIDER_CONFIG`, else
//!   `~/.diverge/provider/`, names the provider's DIRECTORY, created
//!   if absent.
//! - `<dir>/config.yaml` is read if present; absent means the built-in
//!   defaults. No other name or extension is looked for.
//! - Every path inside `config.yaml` resolves relative to `<dir>`,
//!   never to the working directory.
//! - `<dir>/logs/`, `<dir>/data/` and `<dir>/run/` sit beside it;
//!   `data` is the one the file may move.
