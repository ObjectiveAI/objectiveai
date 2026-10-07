//! Dialling the database, over TLS when the mode says.

use bytes::BytesMut;
use postgres_protocol::message::frontend;
use tokio::io::{AsyncRead, AsyncReadExt as _, AsyncWrite, AsyncWriteExt as _};
use tokio::net::TcpStream;

use super::tls;
use crate::database::{Ssl, Target};

/// A dialled database: a socket, plain or wrapped in TLS, read and
/// written as one.
pub type Server = Box<dyn Connected>;

/// What a socket to the database is, either way.
pub trait Connected: AsyncRead + AsyncWrite + Send + Unpin {}

impl<T: AsyncRead + AsyncWrite + Send + Unpin> Connected for T {}

/// Dial the target's host and port, and secure the socket as the
/// mode says: `disable` never; `allow` and `prefer` when the server
/// answers the `SSLRequest` with `S`, plain when `N`; `require`,
/// `verify-ca` and `verify-full` only over TLS, or not at all.
/// Whether the socket is encrypted comes back with it, since
/// cleartext authentication is allowed over TLS alone.
pub async fn dial(target: &Target) -> Result<(Server, bool), String> {
    let tcp = TcpStream::connect((target.host.as_str(), target.port))
        .await
        .map_err(|error| format!("{}:{} could not be dialled: {error}", target.host, target.port))?;
    let _ = tcp.set_nodelay(true);
    if target.ssl == Ssl::Disable {
        return Ok((Box::new(tcp), false));
    }
    let mut tcp = tcp;
    let mut request = BytesMut::new();
    frontend::ssl_request(&mut request);
    tcp.write_all(&request).await.map_err(|error| error.to_string())?;
    let mut answer = [0u8; 1];
    tcp.read_exact(&mut answer).await.map_err(|error| error.to_string())?;
    match (answer[0], target.ssl) {
        (b'S', _) => {
            let secured = tls::secure(tcp, target).await?;
            Ok((Box::new(secured), true))
        }
        (b'N', Ssl::Allow | Ssl::Prefer) => Ok((Box::new(tcp), false)),
        (b'N', _) => Err("the database does not offer TLS, and the URL requires it".to_string()),
        (other, _) => Err(format!("the database answered the SSLRequest with {other:#04x}")),
    }
}
