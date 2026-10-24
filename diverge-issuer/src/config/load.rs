//! Reading the file, and checking the issuer it names.

use std::io;
use std::path::Path;

use url::Url;

use super::Config;
use super::Error;

/// The name the file has, and the only one looked for.
pub const FILE: &str = "issuer.yaml";

/// The configuration at `root`: `issuer.yaml` read and parsed, or the
/// defaults where there is no such file. A file that does not parse,
/// or names a key the document does not have, is [`Error::Parse`],
/// with the path to the value that was wrong; an issuer identifier
/// that is not one is [`Error::Issuer`] — see [`check`].
pub async fn load(root: &Path) -> Result<Config, Error> {
    let file = root.join(FILE);
    let config: Config = match tokio::fs::read(&file).await {
        Ok(bytes) => serde_path_to_error::deserialize(serde_yaml_ng::Deserializer::from_slice(&bytes))
            .map_err(|source| Error::Parse { path: file, source })?,
        Err(error) if error.kind() == io::ErrorKind::NotFound => Config::default(),
        Err(source) => return Err(Error::Io { path: file, source }),
    };
    check(&config.issuer())?;
    Ok(config)
}

/// Whether `issuer` is an issuer identifier, as OpenID Connect
/// Discovery states one:
///
/// 1. It parses as an absolute URL.
/// 2. Its scheme is `https`, or `http` with a loopback host —
///    `127.0.0.1`, `localhost` or `[::1]` — where no TLS is needed.
/// 3. It has no query and no fragment.
pub fn check(issuer: &str) -> Result<(), Error> {
    let refuse = |reason| Error::Issuer { value: issuer.to_string(), reason };
    let url = Url::parse(issuer).map_err(|_| refuse("it is not an absolute URL"))?;
    let loopback = matches!(url.host_str(), Some("127.0.0.1" | "localhost" | "[::1]"));
    match url.scheme() {
        "https" => {}
        "http" if loopback => {}
        _ => return Err(refuse("its scheme is not https, nor http on a loopback host")),
    }
    if url.query().is_some() || url.fragment().is_some() {
        return Err(refuse("it has a query or a fragment"));
    }
    Ok(())
}
