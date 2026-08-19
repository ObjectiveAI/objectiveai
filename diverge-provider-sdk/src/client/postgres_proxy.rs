//! Splicing a plugin's database connection onto a real one.

use std::future::Future;
use std::pin::Pin;

use bytes::Bytes;
use futures_util::Stream;
use tokio::sync::mpsc::UnboundedReceiver;

/// What connects a plugin to the caller's database.
///
/// A provider opens a Postgres channel because something inside a
/// container dialled the conduit it was given. The database lives with
/// the caller, so the bytes come out and this is what splices the far
/// end onto the real thing.
///
/// A plugin that never connects means the channel never exists, which is
/// what makes an opted-out plugin cost nothing rather than cost an idle
/// tunnel.
///
/// # It is the one that is not a request
///
/// The other proxies answer something. This one is handed a socket. A
/// Postgres session is a long-lived conversation with no natural
/// top-level unit — successive frames on the channel are successive
/// writes, and a message larger than one frame simply spans several — so
/// one channel is one connection for its whole life, and this is called
/// once to open it and never again.
///
/// # Never parsed
///
/// Which is what lets TLS negotiation and every protocol extension cross
/// untouched. A conduit that understood pgwire would have to keep up
/// with pgwire; one that does not is finished being written.
///
/// # Failure is a message, not a [`Result`]
///
/// There is no error variant on this channel and this returns no
/// [`Option`], because pgwire already says how a connection goes wrong.
/// A proxy that cannot reach the database sends an `ErrorResponse`
/// (`'E'`) as its first item and ends the stream, which is precisely
/// what the plugin's driver would see from a real server refusing it.
/// Ending the stream without saying anything is the other honest answer,
/// and is what a socket that dropped looks like.
///
/// An [`Option`] would be a third thing to check that collapses to one
/// of those two on the wire — both arms produce the same frames — so it
/// would be a distinction with no consequence.
///
/// Which is what each of these traits does, into a different vocabulary
/// each time — [`OciProxy`](super::oci_proxy::OciProxy) answers in HTTP
/// statuses, [`CommandProxy`](super::command_proxy::CommandProxy) in the
/// CLI's own, and this one in pgwire.
pub trait PostgresProxy: Send + Sync {
    /// Open one connection, and splice it onto the channel.
    ///
    /// Everything the plugin writes arrives on `from_plugin`; everything
    /// the database says goes back on the returned stream. The channel
    /// finishes when that stream ends.
    ///
    /// # What is on `from_plugin`
    ///
    /// Payload bytes, with the frame header and the variant tag already
    /// off them — what the plugin wrote and nothing else. Handing them
    /// on unmodified is the whole job; anything prepended is ten bytes
    /// of garbage in front of a startup message.
    ///
    /// They are not messages. A pgwire message may span several items
    /// and several may share one, exactly as they would arriving off a
    /// socket. A driver on the far end is already prepared for that;
    /// nothing here needs to be.
    ///
    /// # Why it takes the receiver rather than returning a sink
    ///
    /// Because opening a connection is a network dial and the frames
    /// arrive on a queue that cannot wait for one. Taking the receiving
    /// end lets a dispatcher make the queue before it awaits anything,
    /// hand it over, and push the plugin's second write into it while
    /// this is still connecting — with the ordering the queue gives for
    /// free. A sink coming back would have nowhere to put the first
    /// write until this resolved, so it would have to grow this same
    /// queue in front of itself.
    ///
    /// The nearest alternative is taking a sender as a second argument
    /// and returning nothing, and it is a real one: no boxed stream, no
    /// [`Sync`] to satisfy. It loses on shape. Every proxy here returns
    /// what goes back, and that signature's future would instead be the
    /// connection's entire lifetime — a task rather than an answer,
    /// which a dispatcher would have to hold and account for
    /// differently from the other two.
    ///
    /// # `None` does not mean the plugin hung up
    ///
    /// It cannot. Nothing in this protocol lets a REQUESTOR say it is
    /// finished — only a responder finishes — so there is no frame for
    /// a plugin closing its half, and a pgwire half-close arrives as a
    /// `Terminate` (`'X'`) message inside the bytes like anything else.
    ///
    /// `None` means the scope ended or the connection died. It is the
    /// signal to hang up on the database, not a message from the plugin.
    ///
    /// The other direction is not the mirror of this: the returned
    /// stream ending IS representable, and becomes a finish on the
    /// channel that says the database connection dropped.
    ///
    /// # Dropping the receiver is a real answer
    ///
    /// A queue with no reader grows, and nothing in this crate bounds
    /// it, so a proxy that has stopped caring should drop the receiver
    /// rather than hold it. That makes the dispatcher's writes fail,
    /// which is how it learns to finish the channel.
    ///
    /// # The future is [`Send`]
    ///
    /// Because a provider serves several plugins and a plugin may open
    /// several connections, so these overlap by design. It is spelled
    /// out rather than left to `async fn`, which promises nothing about
    /// the future it returns.
    ///
    /// # The bounds on the stream
    ///
    /// [`Send`] and `'static` because it is polled from wherever the
    /// answer is written, which is not where it was built. [`Sync`]
    /// because it is held behind a shared reference while that happens —
    /// the strictest of the three, and the one most likely to bite,
    /// since a stream needs only `&mut` to be polled.
    ///
    /// Written out rather than aliased, as in the two sibling proxies
    /// that return one. An alias would hide exactly those bounds from
    /// the signature a reader is checking theirs against.
    fn connect(
        &self,
        from_plugin: UnboundedReceiver<Bytes>,
    ) -> impl Future<
        Output = Pin<Box<dyn Stream<Item = Bytes> + Send + Sync + 'static>>,
    > + Send;
}
