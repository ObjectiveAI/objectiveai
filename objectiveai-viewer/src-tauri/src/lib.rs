#[cfg(feature = "cli")]
mod api_call;
#[cfg(feature = "cli")]
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
#[cfg(feature = "cli")]
#[doc(hidden)]
pub mod test_internals {
    pub use crate::api_call::api_call_run_impl;
    pub use crate::cli_command::cli_run_impl;
}
