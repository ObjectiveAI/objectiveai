//! The loopback routes, as one HTTP exchange over a loopback socket
//! each: the stop of every program, and the issuer's rotation.

use std::io;

use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
use tokio::net::TcpStream;

use super::Error;

/// `POST <path>` put to the program on `port`, on this host, with the
/// form `body`, and its answer read: the status code and the body.
///
/// - The socket is to `127.0.0.1:<port>`: the routes are honoured only
///   from a loopback peer, and this is one. A connection the host
///   refuses is [`Error::NotListening`] — the program is not running,
///   or is still starting and has no listener yet — and any other
///   failure to connect is [`Error::Connect`].
/// - The request asks the server to close the connection once it has
///   answered, and the answer is read to that close. A socket that
///   fails under the write or the read is [`Error::Exchange`].
/// - The status line is the first line of the answer, `HTTP/1.1
///   <code>`; a server that closes before one is [`Error::NoAnswer`],
///   and a first line that is not a status line is [`Error::Answer`].
///   The body is everything after the blank line, as text; no header
///   is read.
///
/// Nothing is written by hand that an HTTP crate would do better:
/// one request, one status line, no header read, a body taken whole.
pub async fn post(port: u16, path: &str, body: &str) -> Result<(u16, String), Error> {
    let mut socket = match TcpStream::connect(("127.0.0.1", port)).await {
        Ok(socket) => socket,
        Err(error) if error.kind() == io::ErrorKind::ConnectionRefused => {
            return Err(Error::NotListening(port));
        }
        Err(source) => return Err(Error::Connect { port, source }),
    };
    let request = format!(
        "POST {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\nContent-Type: application/x-www-form-urlencoded\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    );
    socket
        .write_all(request.as_bytes())
        .await
        .map_err(Error::Exchange)?;
    socket.flush().await.map_err(Error::Exchange)?;
    let mut answer = Vec::new();
    socket
        .read_to_end(&mut answer)
        .await
        .map_err(Error::Exchange)?;
    let text = String::from_utf8_lossy(&answer).into_owned();
    let code = status(&text)?;
    let body = match text.split_once("\r\n\r\n") {
        Some((_, body)) => body.to_string(),
        None => String::new(),
    };
    Ok((code, body))
}

/// `POST /stop` put to the program on `port`: the server answers `204`
/// and closes the connection as it drains, and that close is the
/// signal the caller reads — the stop accepted, the teardown begun. A
/// `403` is the request refused as not from this host,
/// [`Error::Refused`], and any other code is [`Error::Status`].
pub async fn stop(port: u16) -> Result<(), Error> {
    match post(port, "/stop", "").await?.0 {
        204 => Ok(()),
        403 => Err(Error::Refused),
        code => Err(Error::Status(code)),
    }
}

/// The status code of `answer`'s status line, as [`post`] states it.
fn status(answer: &str) -> Result<u16, Error> {
    let Some(line) = answer.lines().next() else {
        return Err(Error::NoAnswer);
    };
    let mut words = line.split_whitespace();
    let version = words.next().unwrap_or_default();
    match words.next().and_then(|code| code.parse::<u16>().ok()) {
        Some(code) if version.starts_with("HTTP/") => Ok(code),
        _ => Err(Error::Answer(line.to_string())),
    }
}
