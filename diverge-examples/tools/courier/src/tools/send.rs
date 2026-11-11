//! Sending a message: what `create_conversation` and `reply` share.

use crate::caller::Participant;
use crate::courier::Call;
use crate::failure::Failure;
use crate::store;

/// Where a message goes: a new conversation on a topic, or one that
/// exists.
pub enum Into {
    /// Open a conversation with `to` on `topic`.
    New { topic: String },
    /// The conversation with this id.
    Existing(i64),
}

/// What a send made.
pub struct Sent {
    /// The conversation the message is in.
    pub conversation: i64,
    /// The message's id.
    pub message: i64,
}

/// Send `text` from the caller to `to`, into `into`.
///
/// One transaction: the conversation opened if it is new, the message
/// added, the sender's read mark moved past it, and then the
/// notification sent to `to`. A notification that cannot be sent
/// fails the send, and the transaction is rolled back, so nothing is
/// kept; once it is sent the transaction commits.
pub async fn send(call: &Call<'_>, to: &Participant, into: Into, text: &str) -> Result<Sent, Failure> {
    let pool = call.state.store.pool(&call.state.client).await?;
    let mut tx = pool.begin().await?;
    let conversation = match into {
        Into::New { topic } => store::conversations::open(&mut tx, call.template, &topic, &call.caller, to).await?,
        Into::Existing(id) => id,
    };
    let message = store::messages::add(&mut tx, conversation, &call.caller, text).await?;
    call.state.daemon.notify(&call.state.client, to).await?;
    tx.commit().await?;
    Ok(Sent { conversation, message })
}
