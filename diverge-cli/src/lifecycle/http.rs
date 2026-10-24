//! The stop, as one HTTP exchange over a loopback socket.

use std::io;

use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
use tokio::net::TcpStream;

use super::Error;

/// The request, whole: an empty `POST /stop` that asks the server to
/// close the connection once it has answered.
const REQUEST: &[u8] =
    b"POST /stop HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\nContent-Length: 0\r\n\r\n";

/// `POST /stop` put to the program on `port`, on this host, and its
/// answer read.
///
/// - The socket is to `127.0.0.1:<port>`: the route is honoured only
///   from a loopback peer, and this is one. A connection the host
///   refuses is [`Error::NotListening`] — the program is not running,
///   or is still starting and has no listener yet — and any other
///   failure to connect is [`Error::Connect`].
/// - The request is the one above, written whole; the answer is read
///   to the server's close, since the server closes the connection
///   as it drains, and that close is the signal the caller reads: the
///   stop was accepted and the teardown has begun. A socket that
///   fails under the write or the read is [`Error::Exchange`].
/// - The status line is the first line of the answer, `HTTP/1.1
///   <code>`: `204` is the stop accepted, `403` is the request refused
///   as not from this host — [`Error::Refused`] — and any other code
///   is [`Error::Status`]. A server that closes before a status line
///   is [`Error::NoAnswer`], and a first line that is not a status
///   line is [`Error::Answer`].
///
/// Nothing is written by hand that an HTTP crate would do better:
/// one request, one status line, no header read, no body.
pub async fn stop(port: u16) -> Result<(), Error> {
    let mut socket = match TcpStream::connect(("127.0.0.1", port)).await {
        Ok(socket) => socket,
        Err(error) if error.kind() == io::ErrorKind::ConnectionRefused => {
            return Err(Error::NotListening(port));
        }
        Err(source) => return Err(Error::Connect { port, source }),
    };
    socket.write_all(REQUEST).await.map_err(Error::Exchange)?;
    socket.flush().await.map_err(Error::Exchange)?;
    let mut answer = Vec::new();
    socket
        .read_to_end(&mut answer)
        .await
        .map_err(Error::Exchange)?;
    status(&answer)
}

/// The status line of `answer`, judged as the rules above state.
fn status(answer: &[u8]) -> Result<(), Error> {
    let text = String::from_utf8_lossy(answer);
    let Some(line) = text.lines().next() else {
        return Err(Error::NoAnswer);
    };
    let mut words = line.split_whitespace();
    let version = words.next().unwrap_or_default();
    let code = words.next().and_then(|code| code.parse::<u16>().ok());
    match code {
        Some(code) if version.starts_with("HTTP/") => match code {
            204 => Ok(()),
            403 => Err(Error::Refused),
            code => Err(Error::Status(code)),
        },
        _ => Err(Error::Answer(line.to_string())),
    }
}
