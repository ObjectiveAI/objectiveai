//! What the daemon knows of its database: where, as whom, which
//! database, and how to secure the dial.

use std::path::PathBuf;

use diverge_sdk::daemon::endpoints::postgres::Mode;

/// The URL's `sslmode`, as libpq reads it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ssl {
    /// Never TLS.
    Disable,
    /// TLS if the server offers it; libpq's `allow` tries without
    /// first, and the daemon treats it as `prefer`.
    Allow,
    /// TLS if the server offers it, plain otherwise; the default.
    Prefer,
    /// TLS, or no connection; the server's certificate is not
    /// verified, as libpq has it.
    Require,
    /// TLS with the certificate chain verified against the roots.
    VerifyCa,
    /// TLS with the chain and the host name verified.
    VerifyFull,
}

/// The database the daemon serves, as the daemon reaches it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    /// Whether it is the daemon's own cluster.
    pub local: bool,
    /// The URL the store opened: the admin connection.
    pub url: String,
    /// The host to dial.
    pub host: String,
    /// The port.
    pub port: u16,
    /// The database every scope is made in.
    pub dbname: String,
    /// The `sslmode`.
    pub ssl: Ssl,
    /// The `sslrootcert`, a PEM file, when the URL names one.
    pub root_cert: Option<PathBuf>,
}

/// The URL could not be read as one.
#[derive(Debug)]
pub struct TargetError(pub String);

impl std::fmt::Display for TargetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "the database URL could not be read: {}", self.0)
    }
}

impl std::error::Error for TargetError {}

impl Target {
    /// The target the URL names: `local` is the daemon's own cluster,
    /// whose URL names no database, so the database is the store's,
    /// `diverge`; a remote URL names its own, or, naming none, the
    /// user's, as libpq has it.
    pub fn parse(url: &str, local: bool) -> Result<Target, TargetError> {
        let parsed = url::Url::parse(url).map_err(|error| TargetError(error.to_string()))?;
        let host = parsed.host_str().unwrap_or("localhost").trim_matches(['[', ']']).to_string();
        let port = parsed.port().unwrap_or(5432);
        let user = percent_encoding::percent_decode_str(parsed.username()).decode_utf8_lossy().into_owned();
        let named = parsed.path().trim_start_matches('/');
        let named = percent_encoding::percent_decode_str(named).decode_utf8_lossy().into_owned();
        let dbname = if local {
            "diverge".to_string()
        } else if named.is_empty() {
            user
        } else {
            named
        };
        let mut ssl = Ssl::Prefer;
        let mut root_cert = None;
        for (key, value) in parsed.query_pairs() {
            match key.as_ref() {
                "sslmode" | "ssl-mode" => {
                    ssl = match value.as_ref() {
                        "disable" => Ssl::Disable,
                        "allow" => Ssl::Allow,
                        "prefer" => Ssl::Prefer,
                        "require" => Ssl::Require,
                        "verify-ca" | "verify_ca" => Ssl::VerifyCa,
                        "verify-full" | "verify_full" => Ssl::VerifyFull,
                        other => return Err(TargetError(format!("`{other}` is not an sslmode"))),
                    };
                }
                "sslrootcert" | "ssl-root-cert" => root_cert = Some(PathBuf::from(value.as_ref())),
                _ => {}
            }
        }
        Ok(Target {
            local,
            url: url.to_string(),
            host,
            port,
            dbname,
            ssl,
            root_cert,
        })
    }

    /// The mode as a get answers it: local, or the remote URL with its
    /// password taken out.
    pub fn reported(&self) -> Mode {
        if self.local {
            return Mode::Local;
        }
        let url = match url::Url::parse(&self.url) {
            Ok(mut parsed) => {
                let _ = parsed.set_password(None);
                parsed.to_string()
            }
            Err(_) => self.url.clone(),
        };
        Mode::Remote { url }
    }
}
