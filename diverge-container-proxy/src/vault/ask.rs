//! One vault ask, and its one answer.

use axum::body::Bytes;
use diverge_provider_sdk::container_proxy::requests::request::Request;

use crate::requests::{Event, Requests};

/// Ask once and take the answer: the first message on the answer
/// path, then the clean close. Extras are ignored rather than
/// obeyed; a path that dies, or closes with nothing, is the failure
/// it is. No retry, per the wire: a vault operation is not safe to
/// repeat.
pub async fn ask(requests: &Requests, request: Request<'_>) -> Result<Bytes, Asked> {
    let Ok((_, mut receiver)) = requests.ask(request).await else {
        return Err(Asked::Encode);
    };
    let mut answer: Option<Bytes> = None;
    loop {
        match receiver.recv().await {
            Some(Event::Message(bytes)) => {
                answer.get_or_insert(bytes);
            }
            Some(Event::Complete) => {
                return answer.ok_or(Asked::Empty);
            }
            Some(Event::Died) | None => return Err(Asked::Died),
            // The postgres path's alone; never on a vault path.
            Some(Event::Opened(_)) => {}
        }
    }
}

/// Why no answer came.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Asked {
    /// The ask would not encode — the proxy's own fault.
    Encode,
    /// The answer path closed with no message.
    Empty,
    /// The answer path died, or `/requests` did before it opened.
    Died,
}
