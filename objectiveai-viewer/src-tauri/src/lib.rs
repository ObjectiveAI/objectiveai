mod cli_command;
mod plugins;
#[cfg(test)]
mod plugins_tests;
mod run;
mod signature;

pub use plugins::*;
pub use run::*;

/// Internal command bodies exposed for integration testing. Off the
/// normal public API — integration tests call these to drive the
/// bridge without constructing a `tauri::State`. `#[doc(hidden)]`
/// on the inner items keeps them out of rustdoc.
#[doc(hidden)]
pub mod test_internals {
    pub use crate::cli_command::cli_run_impl;
}
