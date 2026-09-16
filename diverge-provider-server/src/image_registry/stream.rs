//! A blob on its way through, hashed as it goes.

use std::io;

use bytes::Bytes;
use diverge_provider_sdk::server::image_source::BlobStream;
use futures_util::{Stream, StreamExt as _, stream};

use super::{Digest, Hasher};

/// What a hash-as-you-go stream carries between pieces: the caller's
/// pieces, the hash so far — `None` once the blob has ended and the
/// stream with it — the one piece held back, and the digest it all
/// must come to.
struct Passing {
    pieces: BlobStream,
    hasher: Option<Hasher>,
    held: Option<Bytes>,
    digest: Digest,
}

/// The blob's pieces as the response body, each hashed on its way
/// through, the blob completed only if the hash comes out.
///
/// One piece is always held back: when a piece arrives it is hashed
/// and the PREVIOUS piece is yielded, so the last piece of the blob
/// is still in hand when the caller's stream ends. Then the hash is
/// finished — the held piece is yielded if it is the digest's, and an
/// error is yielded if it is not, which hyper turns into a closed
/// connection and podman into a failed pull. A caller that goes away
/// mid-blob leaves a stream that ends short, which hashes wrong and
/// ends the same way. What is ever in memory is one piece, at most
/// [`CHUNK_SIZE`](diverge_provider_sdk::CHUNK_SIZE), per blob in
/// flight. Nothing empty is ever yielded: the first piece is held
/// without a yield, and a blob of no pieces yields its verdict alone.
pub fn verified(digest: Digest, pieces: BlobStream) -> impl Stream<Item = Result<Bytes, io::Error>> {
    let hasher = Some(digest.hasher());
    stream::unfold(
        Passing {
            pieces,
            hasher,
            held: None,
            digest,
        },
        |mut passing| async move {
            let mut hasher = passing.hasher.take()?;
            loop {
                match passing.pieces.next().await {
                    Some(piece) => {
                        hasher.update(&piece);
                        if let Some(previous) = passing.held.replace(piece) {
                            passing.hasher = Some(hasher);
                            return Some((Ok(previous), passing));
                        }
                    }
                    None => {
                        let held = passing.held.take().unwrap_or_default();
                        let outcome = if passing.digest.is(&hasher.finish()) {
                            Ok(held)
                        } else {
                            Err(io::Error::other(format!("the blob's bytes are not {}", passing.digest)))
                        };
                        return Some((outcome, passing));
                    }
                }
            }
        },
    )
}
