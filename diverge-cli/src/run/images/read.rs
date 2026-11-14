//! `diverge run images read`: whether the daemon can supply an image, and,
//! where it can, one ask of it — its DIVERGE.md, a manifest, a blob, or
//! the proof — as [`read`](diverge_sdk::daemon::endpoints::images::read)
//! states it.

use std::path::PathBuf;

use clap::{ArgGroup, Args};
use diverge_sdk::daemon::endpoints::images::read::{client::execute, client::request, server::response};
use diverge_sdk::provider::endpoints::containers::client::UnaryError;
use diverge_sdk::shared::containers::info::Document;
use diverge_sdk::shared::containers::oci;
use futures_util::StreamExt as _;
use serde::Serialize;
use tokio::io::AsyncWriteExt as _;

use crate::run::{Below, Emitter, Error, Target, body, request_schema, response_schema};

/// Whether the daemon can supply an image, and one ask of it where it
/// can.
#[derive(Debug, Clone, PartialEq, Eq, Args)]
#[command(subcommand_negates_reqs = true, args_conflicts_with_subcommands = true)]
#[command(group(ArgGroup::new("ask").args(["info", "manifest", "blob", "prove_nonce"]).multiple(false)))]
pub struct Arguments {
    /// What the request carries, as a JSON object: see
    /// [`request::Frame`] — the image, and the remote daemons not to
    /// ask; `relay` among its members has another daemon serve it.
    #[arg(value_name = "BODY")]
    pub body: Option<String>,
    /// Once held, the image's DIVERGE.md, printed parsed: its name, the
    /// schema of its arguments, what it says of itself, and its body.
    #[arg(long)]
    pub info: bool,
    /// Once held, the manifest under this digest the image reaches,
    /// printed with its media type.
    #[arg(long, value_name = "DIGEST")]
    pub manifest: Option<String>,
    /// Once held, the blob under this digest the image reaches, written
    /// to `--out`.
    #[arg(long, value_name = "DIGEST", requires = "out")]
    pub blob: Option<String>,
    /// The file `--blob`'s bytes are written to, truncated first.
    #[arg(long, value_name = "PATH", requires = "blob")]
    pub out: Option<PathBuf>,
    /// Once held, the proof over the image under this nonce — 64
    /// hexadecimal characters — printed as 64 lowercase hexadecimal
    /// characters.
    #[arg(long, value_name = "HEX", value_parser = nonce)]
    pub prove_nonce: Option<[u8; 32]>,
    /// With `--prove-nonce`, the digest of the one manifest in the
    /// image's index whose image is meant; absent when the image's
    /// digest names an image manifest.
    #[arg(long, value_name = "DIGEST", requires = "prove_nonce")]
    pub platform: Option<String>,
    /// What is beneath the request: `request-schema`, the JSON Schema
    /// of the body it takes, or `response-schema`, the JSON Schema of
    /// every line it prints — printed in place of putting any request.
    #[command(subcommand)]
    pub below: Option<Below>,
}

/// What the leaf prints, one line each: the daemon's answer first, and
/// — when it is `held` and an ask was given — the ask's answer.
/// Untagged, so the line is the one or the other as it is: `"held"`,
/// `{"error":…}`, `{"info":…}`, `"missing"`, `{"media_type":…,"body":…}`,
/// `{"path":…,"bytes":N}`, `{"proof":…}`, `"not_held"`.
#[derive(Debug, Clone, PartialEq, Serialize, schemars::JsonSchema)]
#[serde(untagged)]
pub enum Line {
    /// The daemon's answer to whether it can supply the image.
    Answer(response::Frame),
    /// The image's DIVERGE.md asked for, parsed.
    Info {
        /// The document.
        info: Document,
    },
    /// The image ships no DIVERGE.md: see [`Shipped`].
    Shipped(Shipped),
    /// The manifest asked for.
    Manifest {
        /// Its media type.
        media_type: String,
        /// Its bytes, as text — a manifest is JSON; anything that is
        /// not UTF-8 is replaced.
        body: String,
    },
    /// The blob asked for, written.
    Written {
        /// The file it was written to.
        path: PathBuf,
        /// How many bytes it holds.
        bytes: u64,
    },
    /// The proof asked for.
    Proof {
        /// The 32 bytes, as 64 lowercase hexadecimal characters.
        proof: String,
    },
    /// The ask answered with the empty finish: see [`Missing`].
    Missing(Missing),
}

/// What an image that ships no DIVERGE.md is answered with: a fact
/// about the image, and not a failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Shipped {
    /// The image ships none.
    Missing,
}

/// An ask the source did not answer: a digest the image does not reach,
/// or a proof it would not give — the empty finish, said.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Missing {
    /// Nothing came before the finish.
    NotHeld,
}

/// The request put and its answer printed; where it is `held`, the one
/// ask given put and its answer printed, and the scope cancelled and
/// waited to its end.
pub async fn run(arguments: Arguments, target: &Target<'_>, emitter: &mut Emitter) -> Result<(), Error> {
    match arguments.below {
        Some(Below::RequestSchema(_)) => {
            return request_schema::<request::Frame>("diverge run images read", emitter).await;
        }
        Some(Below::ResponseSchema(_)) => {
            return response_schema::<Line>("diverge run images read", emitter).await;
        }
        None => {}
    }
    let request: request::Frame = body(arguments.body.as_deref())?;
    let handle = target.dial().await?;
    let held = match execute::execute(&handle, &request).await {
        Ok(held) => held,
        Err(execute::ExecuteError::NotFound) => return emit(emitter, Line::Answer(response::Frame::NotFound)).await,
        Err(execute::ExecuteError::Forbidden) => return emit(emitter, Line::Answer(response::Frame::Forbidden)).await,
        Err(execute::ExecuteError::Daemon(error)) => {
            return emit(emitter, Line::Answer(response::Frame::Error(error))).await;
        }
        Err(execute::ExecuteError::Closed | execute::ExecuteError::Send(_)) => return Err(Error::Rejected),
        Err(error) => return Err(Error::Exchange(error.to_string())),
    };
    emit(emitter, Line::Answer(response::Frame::Held)).await?;
    let outcome = ask(&held, arguments).await;
    let _ = held.cancel().await;
    held.wait().await;
    match outcome? {
        Some(line) => emit(emitter, line).await,
        None => Ok(()),
    }
}

/// The one ask given, put on the held scope: its line, or none where no
/// ask was given.
async fn ask(held: &execute::ExecuteHandle, arguments: Arguments) -> Result<Option<Line>, Error> {
    if arguments.info {
        return Ok(Some(match held.info().await {
            Ok(Some(info)) => Line::Info { info },
            Ok(None) => Line::Shipped(Shipped::Missing),
            Err(UnaryError::Refused(error)) => Line::Answer(response::Frame::Error(error)),
            Err(error) => return Err(Error::Exchange(error.to_string())),
        }));
    }
    if let Some(digest) = arguments.manifest {
        let manifest = held.manifest(&digest).await.map_err(|error| Error::Exchange(error.to_string()))?;
        return Ok(Some(match manifest {
            Some(manifest) => Line::Manifest {
                media_type: manifest.media_type,
                body: String::from_utf8_lossy(&manifest.body).into_owned(),
            },
            None => Line::Missing(Missing::NotHeld),
        }));
    }
    if let (Some(digest), Some(path)) = (arguments.blob, arguments.out) {
        return blob(held, &digest, path).await.map(Some);
    }
    if let Some(nonce) = arguments.prove_nonce {
        let request = oci::prove::request::Request {
            platform: arguments.platform,
            nonce,
        };
        let proof = held.prove(&request).await.map_err(|error| Error::Exchange(error.to_string()))?;
        return Ok(Some(match proof {
            Some(proof) => Line::Proof { proof: hex(&proof) },
            None => Line::Missing(Missing::NotHeld),
        }));
    }
    Ok(None)
}

/// The blob under `digest` written to `path`, every piece appended as it
/// comes, the file truncated first. Zero pieces is a digest the image
/// does not reach: the file is left empty, and the line says so.
async fn blob(held: &execute::ExecuteHandle, digest: &str, path: PathBuf) -> Result<Line, Error> {
    let mut pieces = held.blob(digest).await.map_err(|error| Error::Exchange(error.to_string()))?;
    let mut file = tokio::fs::File::create(&path).await.map_err(|source| Error::Io {
        path: path.clone(),
        source,
    })?;
    let mut bytes: u64 = 0;
    let mut any = false;
    while let Some(piece) = pieces.next().await {
        let piece = piece.map_err(|error| Error::Exchange(error.to_string()))?;
        file.write_all(&piece).await.map_err(|source| Error::Io {
            path: path.clone(),
            source,
        })?;
        bytes += piece.len() as u64;
        any = true;
    }
    file.flush().await.map_err(|source| Error::Io {
        path: path.clone(),
        source,
    })?;
    if !any {
        return Ok(Line::Missing(Missing::NotHeld));
    }
    Ok(Line::Written { path, bytes })
}

/// One line printed.
async fn emit(emitter: &mut Emitter, line: Line) -> Result<(), Error> {
    emitter.emit(&line).await.map(|_| ())
}

/// `bytes` as lowercase hexadecimal, two characters a byte.
fn hex(bytes: &[u8]) -> String {
    let mut text = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        text.push(char::from_digit(u32::from(byte >> 4), 16).expect("a nibble is a hex digit"));
        text.push(char::from_digit(u32::from(byte & 0x0f), 16).expect("a nibble is a hex digit"));
    }
    text
}

/// `--prove-nonce`: exactly 64 hexadecimal characters, of either case,
/// as the 32 bytes they spell.
fn nonce(text: &str) -> Result<[u8; 32], String> {
    let digits = text.as_bytes();
    if digits.len() != 64 {
        return Err("a nonce is 64 hexadecimal characters".to_string());
    }
    let mut nonce = [0u8; 32];
    for (byte, pair) in nonce.iter_mut().zip(digits.chunks_exact(2)) {
        let high = char::from(pair[0]).to_digit(16);
        let low = char::from(pair[1]).to_digit(16);
        let (Some(high), Some(low)) = (high, low) else {
            return Err("a nonce is 64 hexadecimal characters".to_string());
        };
        *byte = (high * 16 + low) as u8;
    }
    Ok(nonce)
}
