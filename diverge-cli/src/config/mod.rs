//! The one `cli.yaml` the CLI reads.
//!
//! - The ROOT is `--config <dir>`, else `DIVERGE_CONFIG`, else
//!   `.diverge` in the working directory where that directory exists,
//!   else `.diverge` under the home directory, made if absent — see
//!   [`root`]. All but the working directory's turn is what every
//!   Diverge program does; that turn is the CLI's own, so that a
//!   project carrying its own `.diverge` is dialled from inside it.
//!   A `daemon start` or a `provider start` hands this root to the
//!   program it spawns as `--config <root>`, so the program runs on
//!   the root the CLI found, the working directory's turn included —
//!   see [`lifecycle`](crate::lifecycle).
//! - `<root>/cli.yaml` is read if present; absent means the built-in
//!   default, [`Config::default`]. No other name is looked for — see
//!   [`load`].
//! - The document is [`Config`]: `daemons`, by name, each a
//!   [`Daemon`] — its `address`, its `accounts` by name, each an
//!   [`Account`]: the `issuer` and `subject` of one of the root's
//!   identities, the account the daemon admits — and its `default`
//!   account, where it has one — and the `default` daemon. A key the document does not have is
//!   refused, so a misspelled setting is refused rather than ignored.
//! - [`select`] picks the daemon and the account a run is: the ones
//!   `run` names, else the defaults, each refused by name when the
//!   document has no such one, and an account refused when none is
//!   named and the daemon has no default; [`daemon`] picks the daemon
//!   alone.
//!
//! ```yaml
//! default: local
//! daemons:
//!   local:
//!     address: ws://127.0.0.1:14980/
//! ```
//!
//! That document is the built-in default, word for word: the daemon
//! on this host at its own default port, with no account, since a
//! fresh daemon admits no one. An account is added beside it:
//!
//! ```yaml
//! daemons:
//!   local:
//!     address: ws://127.0.0.1:14980/
//!     default: me
//!     accounts:
//!       me:
//!         issuer: https://auth.example.com
//!         subject: alice
//! ``` [`Error`] is why the root or the file could not be
//! used, or the names could not be found.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod account;
mod config;
mod daemon;
mod error;
mod load;
mod root;
mod select;

pub use account::*;
pub use config::*;
pub use daemon::*;
pub use error::*;
pub use load::*;
pub use root::*;
pub use select::*;
