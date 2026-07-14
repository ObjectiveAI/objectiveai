//! In-process [`CommandExecutor`] implementor backed by the CLI's own
//! leaf handlers. Lets programmatic consumers drive the SDK's per-leaf
//! `execute<E>` entry points against the CLI's local logic without
//! spawning a subprocess.
//!
//! `run.rs` also delegates here for non-`instance` argv — the
//! `objectiveai_sdk::cli::command::execute(&executor, request)` call
//! flows back through this executor down to each leaf.

use std::any::{Any, TypeId};
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context as TaskContext, Poll};

use futures::{Stream, StreamExt, TryStreamExt};
use objectiveai_sdk::cli::command::{
    AgentArguments, CommandExecutor, CommandRequest, CommandResponse, ResponseItem, Transform,
    parse_request,
};
use serde_json::Value;

use crate::context::Context;
use crate::error::Error;

/// In-process executor. Owns a [`Context`] and dispatches each
/// `CommandRequest` through the CLI's local root dispatcher.
pub struct DaemonCommandExecutor {
    ctx: Context,
    /// Broadcast tee: every PRE-transform response item is serialized
    /// and sent here (unbounded — a send never blocks stream
    /// yielding); the writer task `run()` spawned drains it onto the
    /// daemon's producer socket, and `run()` awaits that writer's
    /// completion before its returned stream finishes. `None` = this
    /// execution is not broadcast (nested/internal executors).
    tee: Option<tokio::sync::mpsc::UnboundedSender<Value>>,
}

impl DaemonCommandExecutor {
    pub fn new(
        ctx: Context,
        tee: Option<tokio::sync::mpsc::UnboundedSender<Value>>,
    ) -> Self {
        Self { ctx, tee }
    }
}

/// Walk down externally-tagged enum wrappers (`{"<Variant>": <inner>}`)
/// until `from_value::<T>` succeeds, or no more single-key-object layers
/// remain. The aggregator's variant keys are PascalCase and leaf fields
/// are snake_case across the SDK, so the first level whose shape matches
/// `T` is the leaf.
fn extract_leaf<T: serde::de::DeserializeOwned>(value: Value) -> Result<T, serde_json::Error> {
    let mut current = value;
    loop {
        match serde_json::from_value::<T>(current.clone()) {
            Ok(t) => return Ok(t),
            Err(_) => match current {
                Value::Object(map) if map.len() == 1 => {
                    current = map.into_iter().next().unwrap().1;
                }
                other => return serde_json::from_value::<T>(other),
            },
        }
    }
}

/// Build a per-call [`Context`] from `base`. When `agent_arguments` is
/// `Some`, clone the base ctx and overwrite the seven per-request
/// identity fields on its `Config`: `agent_id`, `agent_full_id`,
/// `agent_remote`, `response_id`, `response_ids`, `mcp_session_id` are
/// set verbatim (including `None`, which clears the slot), and
/// `agent_instance_hierarchy` falls back to `"UNKNOWN"` when missing
/// because it's a non-nullable String on the cli's `Config`. The
/// clone's API-client cell is detached afterwards so the memoized
/// `HttpClient` rebuilds with the overridden identity headers. When
/// `agent_arguments` is `None`, `base` is borrowed unchanged.
///
/// Shared by [`DaemonCommandExecutor`] and the daemon's `/execute`
/// SSE route (`crate::http::daemon_execute`), which applies
/// the same override to its own resident ctx per request.
pub(crate) fn apply_agent_arguments<'a>(
    base: &'a Context,
    agent_arguments: Option<&AgentArguments>,
) -> std::borrow::Cow<'a, Context> {
    match agent_arguments {
        None => std::borrow::Cow::Borrowed(base),
        Some(args) => {
            let mut ctx = base.clone();
            ctx.config.agent_instance_hierarchy = args
                .agent_instance_hierarchy
                .clone()
                .unwrap_or_else(|| "UNKNOWN".to_string());
            ctx.config.agent_id = args.agent_id.clone();
            ctx.config.agent_full_id = args.agent_full_id.clone();
            ctx.config.agent_remote = args.agent_remote.clone();
            ctx.config.response_id = args.response_id.clone();
            ctx.config.response_ids = args.response_ids.clone();
            ctx.config.mcp_session_id = args.mcp_session_id.clone();
            ctx.reset_api_client();
            std::borrow::Cow::Owned(ctx)
        }
    }
}

impl DaemonCommandExecutor {
    /// Build the per-call [`Context`] for this execute — see
    /// [`apply_agent_arguments`].
    fn resolve_ctx<'a>(
        &'a self,
        agent_arguments: Option<&AgentArguments>,
    ) -> std::borrow::Cow<'a, Context> {
        apply_agent_arguments(&self.ctx, agent_arguments)
    }
}

impl CommandExecutor for DaemonCommandExecutor {
    type Error = Error;
    type Stream<T>
        = Pin<Box<dyn Stream<Item = Result<T, Self::Error>> + Send>>
    where
        T: Send + 'static;

    async fn execute<R, T>(
        &self,
        request: R,
        agent_arguments: Option<&AgentArguments>,
    ) -> Result<Self::Stream<T>, Self::Error>
    where
        R: CommandRequest + Send + serde::Serialize,
        T: CommandResponse + serde::Serialize + serde::de::DeserializeOwned + Send + 'static,
    {
        // Pull the envelope controls off the request up front, before
        // the stream is built — upcoming conditional transforms consume
        // these: the output transform (owned: jq / python, python wins),
        // the timeout cap, and the max-tokens cap.
        let base = request.request_base();
        let transform = base.transform();
        let timeout = base.timeout_seconds;
        let max_tokens = base.max_tokens;

        // Round-trip the typed request through the cli's `--request` JSON
        // entry — request -> argv lowering no longer exists. `parse_request`
        // deserializes the JSON straight back into the aggregate `Request`.
        let argv = vec![
            "--request".to_string(),
            serde_json::to_string(&request).map_err(Error::InlineJson)?,
        ];
        let sdk_request = parse_request(&argv).map_err(|e| match e {
            objectiveai_sdk::cli::command::ParseError::Clap(e) => Error::ClapParse(e),
            objectiveai_sdk::cli::command::ParseError::FromArgs(e) => Error::FromArgs(e),
        })?;
        // Own the ctx so the deferred dispatch future can hold it —
        // the returned stream is `'static` and outlives this call.
        let ctx = self.resolve_ctx(agent_arguments).into_owned();
        // The Python transform adapter needs its own ctx handle (for
        // the python runtime); clone one before `ctx` is moved into the
        // dispatch future, but only for a Python transform (jq needs no
        // ctx).
        let transform_ctx =
            matches!(transform, Some(Transform::Python(_))).then(|| ctx.clone());

        // Base stream ("the first one"): don't await the dispatcher
        // here. Wrap the `Future<Result<Stream<Result<ResponseItem>>>>`
        // as a stream that, on first poll, drives the future and
        // flattens into its inner stream — so `execute` returns
        // instantly and a dispatch setup error surfaces as the stream's
        // first item rather than an eager `Err`. Box it so the wrappers
        // below can hold it `Unpin`.
        let source = futures::stream::once(async move {
            crate::command::command::execute(&ctx, sdk_request).await
        })
        .try_flatten();
        let source: Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>> =
            Box::pin(source);

        // Layer the wrapper adapters on top of the base, conditionally.
        //
        // Always: convert root `ResponseItem` items to the caller's `T`
        // ([`ConvertStream`] — identity fast-path when `T ==
        // ResponseItem`, else a serde round-trip).
        let mut stream: Self::Stream<T> = Box::pin(ConvertStream::new(source));

        // Broadcast tee: mirror the typed PRE-transform items onto the
        // daemon's broadcast — the `/listen` wire always carries the
        // leaf-typed items, never transformed output. Sits below the
        // transform/token/timeout adapters, so the caps still bound
        // the broadcast (a stopped downstream stops pulling through
        // the tee).
        if let Some(tee) = self.tee.clone() {
            stream = Box::pin(TeeStream::new(stream, tee));
        }

        // Conditional: an output transform — each item is run through
        // the transform and its (non-null) output replaces it; a null
        // output skips the item. Comes BEFORE the token budget so the
        // budget counts the transformed output. python overrides jq
        // (resolved upstream by `RequestBase::transform`).
        if let Some(transform) = transform {
            match transform {
                Transform::Python(code) => {
                    // A per-stream-item transform must not make nested host
                    // calls — disable `objectiveai.execute` for it automatically.
                    let ctx = transform_ctx
                        .expect("cloned whenever a python transform is present")
                        .with_no_objectiveai(true);
                    stream = Box::pin(PythonTransformStream::new(stream, ctx, code));
                }
                Transform::Jq(filter) => {
                    stream = Box::pin(JqTransformStream::new(stream, filter));
                }
            }
        }

        // Conditional: a running token budget over the serialized output
        // ([`TokenCountStream`]). Sits BEFORE the timeout so the deadline
        // still bounds it.
        if let Some(max_tokens) = max_tokens {
            stream = Box::pin(TokenCountStream::new(stream, max_tokens));
        }

        // Conditional: a single whole-stream deadline ([`TimeoutStream`]
        // — anchored at first poll; the stream never outlives it). Last,
        // so it's the outermost cap over every other adapter.
        if let Some(timeout_seconds) = timeout {
            stream = Box::pin(TimeoutStream::new(stream, timeout_seconds));
        }

        Ok(stream)
    }

    async fn execute_one<R, T>(
        &self,
        request: R,
        agent_arguments: Option<&AgentArguments>,
    ) -> Result<T, Self::Error>
    where
        R: CommandRequest + Send + serde::Serialize,
        T: CommandResponse + serde::Serialize + serde::de::DeserializeOwned + Send + 'static,
    {
        let mut stream: Self::Stream<T> =
            self.execute::<R, T>(request, agent_arguments).await?;
        match stream.next().await {
            Some(item) => item,
            None => Err(Error::EmptyStream),
        }
    }
}

/// Pass-through adapter that mirrors every item onto the broadcast
/// tee. `Ok` items serialize as their JSON; `Err` items serialize as
/// the structured `{type:"error",...}` line (the same payload
/// `main.rs::write_error_line` prints). Sends are unbounded and never
/// block; a send failure (the writer task is gone — daemon down)
/// permanently disables the tee for this execution, never the
/// command.
struct TeeStream<T> {
    inner: Pin<Box<dyn Stream<Item = Result<T, Error>> + Send>>,
    tee: Option<tokio::sync::mpsc::UnboundedSender<Value>>,
}

impl<T> TeeStream<T> {
    fn new(
        inner: Pin<Box<dyn Stream<Item = Result<T, Error>> + Send>>,
        tee: tokio::sync::mpsc::UnboundedSender<Value>,
    ) -> Self {
        Self {
            inner,
            tee: Some(tee),
        }
    }
}

impl<T> Stream for TeeStream<T>
where
    T: serde::Serialize,
{
    type Item = Result<T, Error>;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        let this = self.get_mut();
        let poll = this.inner.as_mut().poll_next(cx);
        if let std::task::Poll::Ready(Some(item)) = &poll {
            if let Some(tee) = &this.tee {
                let json = match item {
                    Ok(value) => serde_json::to_value(value).ok(),
                    Err(e) => serde_json::to_value(objectiveai_sdk::cli::Error {
                        r#type: objectiveai_sdk::cli::ErrorType::Error,
                        level: Some(objectiveai_sdk::cli::Level::Error),
                        fatal: None,
                        message: e.output_message(),
                    })
                    .ok(),
                };
                if let Some(json) = json {
                    if tee.send(json).is_err() {
                        this.tee = None;
                    }
                }
            }
        }
        poll
    }
}

/// Stream adapter: convert the dispatcher's root [`ResponseItem`] items
/// into the caller's `T`.
///
/// Mode is decided once at construction by a `TypeId` compare:
/// - `T == ResponseItem`: identity — move each item straight through
///   with no serialization (the alloc-free `Option` downcast).
/// - otherwise: serialize to `serde_json::Value` and walk the
///   externally-tagged wrappers down to `T` via [`extract_leaf`].
struct ConvertStream<S, T> {
    inner: S,
    /// `true` ⇔ `T` is the root `ResponseItem` (take the identity path).
    identity: bool,
    _marker: PhantomData<fn() -> T>,
}

impl<S, T: 'static> ConvertStream<S, T> {
    fn new(inner: S) -> Self {
        Self {
            inner,
            identity: TypeId::of::<T>() == TypeId::of::<ResponseItem>(),
            _marker: PhantomData,
        }
    }
}

impl<S, T> Stream for ConvertStream<S, T>
where
    S: Stream<Item = Result<ResponseItem, Error>> + Unpin,
    T: serde::de::DeserializeOwned + 'static,
{
    type Item = Result<T, Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        match Pin::new(&mut this.inner).poll_next(cx) {
            Poll::Ready(Some(Ok(item))) => Poll::Ready(Some(convert_item::<T>(item, this.identity))),
            Poll::Ready(Some(Err(e))) => Poll::Ready(Some(Err(e))),
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

/// Convert one root item to `T`, per the mode [`ConvertStream`] chose.
fn convert_item<T: serde::de::DeserializeOwned + 'static>(
    item: ResponseItem,
    identity: bool,
) -> Result<T, Error> {
    if identity {
        let mut slot = Some(item);
        Ok((&mut slot as &mut dyn Any)
            .downcast_mut::<Option<T>>()
            .and_then(Option::take)
            .expect("identity flag guarantees T == ResponseItem"))
    } else {
        let value = serde_json::to_value(item).map_err(Error::InlineJson)?;
        extract_leaf::<T>(value).map_err(Error::InlineJson)
    }
}

/// Stream adapter: a Python output transform with null-skip.
///
/// Each upstream item is fed to the script as the global `input`; the
/// script's output is taken back. NO output (a null result / nothing
/// printed) SKIPS the item (nothing is yielded); any other output is
/// deserialized back into `T` and yielded. Upstream errors pass
/// through. The runtime call is async, so the in-flight future is held
/// across polls (one item transformed at a time).
struct PythonTransformStream<S, T> {
    inner: S,
    ctx: Context,
    code: Arc<str>,
    pending: Option<Pin<Box<dyn Future<Output = Result<Option<serde_json::Value>, Error>> + Send>>>,
    _marker: PhantomData<fn() -> T>,
}

impl<S, T> PythonTransformStream<S, T> {
    fn new(inner: S, ctx: Context, code: String) -> Self {
        Self {
            inner,
            ctx,
            code: Arc::from(code),
            pending: None,
            _marker: PhantomData,
        }
    }
}

impl<S, T> Stream for PythonTransformStream<S, T>
where
    S: Stream<Item = Result<T, Error>> + Unpin,
    T: serde::Serialize + serde::de::DeserializeOwned + Send + 'static,
{
    type Item = Result<T, Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        loop {
            // Drive an in-flight transform to completion first. The
            // script's output arrives as a raw `Value`: a null result
            // (or no output) skips the item; only a non-null `Value` is
            // deserialized into `T` and yielded.
            if let Some(fut) = this.pending.as_mut() {
                match fut.as_mut().poll(cx) {
                    Poll::Ready(Ok(Some(value))) => {
                        this.pending = None;
                        if !value.is_null() {
                            return Poll::Ready(Some(
                                serde_json::from_value::<T>(value).map_err(Error::InlineJson),
                            ));
                        }
                        // null output → skip; fall through to the next item.
                    }
                    // no output → skip; fall through to pull the next item.
                    Poll::Ready(Ok(None)) => this.pending = None,
                    Poll::Ready(Err(e)) => {
                        this.pending = None;
                        return Poll::Ready(Some(Err(e)));
                    }
                    Poll::Pending => return Poll::Pending,
                }
            }
            // No in-flight transform — pull the next upstream item.
            match Pin::new(&mut this.inner).poll_next(cx) {
                Poll::Ready(Some(Ok(item))) => {
                    let ctx = this.ctx.clone();
                    let code = Arc::clone(&this.code);
                    this.pending = Some(Box::pin(async move {
                        transform_item::<T>(&ctx, &code, item).await
                    }));
                    // loop to poll the freshly-built future
                }
                Poll::Ready(Some(Err(e))) => return Poll::Ready(Some(Err(e))),
                Poll::Ready(None) => return Poll::Ready(None),
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}

/// Run the Python transform for one item: feed it as `input`, take the
/// output as a raw `serde_json::Value` (NOT yet `T`). `None` ⇔ the
/// script produced no output. The caller decides skip-vs-yield: a null
/// `Value` (or `None`) skips, and only a non-null `Value` is
/// deserialized into `T` — so a non-nullable `T` never errors on a
/// "skip" result.
async fn transform_item<I: serde::Serialize>(
    ctx: &Context,
    code: &str,
    item: I,
) -> Result<Option<serde_json::Value>, Error> {
    ctx.python().await?.exec_code(ctx, code, Some(item)).await
}

/// Stream adapter: a jq output transform with null-skip.
///
/// Each upstream item is run through the jq `filter` (jaq, via
/// [`crate::filesystem::run_jq`]); the multi-result vector is collapsed
/// the historical way — 0 results → null, exactly 1 → that result
/// AS-IS, >1 → an array. A null collapsed result SKIPS the item (so a
/// jq `select(...)` that yields nothing filters it out); any other
/// result is deserialized into `T` and yielded. Upstream errors pass
/// through. jaq is a synchronous CPU call, so the transform runs inline
/// in `poll_next` — no held future.
struct JqTransformStream<S, T> {
    inner: S,
    filter: Arc<str>,
    _marker: PhantomData<fn() -> T>,
}

impl<S, T> JqTransformStream<S, T> {
    fn new(inner: S, filter: String) -> Self {
        Self {
            inner,
            filter: Arc::from(filter),
            _marker: PhantomData,
        }
    }
}

impl<S, T> Stream for JqTransformStream<S, T>
where
    S: Stream<Item = Result<T, Error>> + Unpin,
    T: serde::Serialize + serde::de::DeserializeOwned,
{
    type Item = Result<T, Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        loop {
            match Pin::new(&mut this.inner).poll_next(cx) {
                Poll::Ready(Some(Ok(item))) => match jq_apply::<T>(&item, &this.filter) {
                    Ok(Some(t)) => return Poll::Ready(Some(Ok(t))),
                    // null collapsed result → skip; loop to the next item.
                    Ok(None) => {}
                    Err(e) => return Poll::Ready(Some(Err(e))),
                },
                Poll::Ready(Some(Err(e))) => return Poll::Ready(Some(Err(e))),
                Poll::Ready(None) => return Poll::Ready(None),
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}

/// Run the jq filter for one item and collapse its results the
/// historical way (length-1 → as-is). `None` ⇔ the collapsed result is
/// null (skip the item); otherwise the result deserialized into `T`.
fn jq_apply<T: serde::Serialize + serde::de::DeserializeOwned>(
    item: &T,
    filter: &str,
) -> Result<Option<T>, Error> {
    let mut results = crate::filesystem::run_jq(item, filter).map_err(Error::Filesystem)?;
    let value = match results.len() {
        0 => serde_json::Value::Null,
        1 => results.remove(0),
        _ => serde_json::Value::Array(results),
    };
    if value.is_null() {
        Ok(None)
    } else {
        serde_json::from_value::<T>(value)
            .map(Some)
            .map_err(Error::InlineJson)
    }
}

/// Stream adapter: a running token budget over the serialized output.
///
/// Each item is `serde_json::to_string`'d and tokenized (tiktoken
/// `o200k_base`, the same encoding the old `db query` budget used); the
/// token count is added to a running total. The moment the total
/// exceeds `max_tokens`, the adapter yields one final
/// `Err(Error::TokenBudgetExceeded)` IN PLACE OF the item that pushed
/// it over, and then ends. The encoder is built once at construction.
struct TokenCountStream<S> {
    inner: S,
    max_tokens: u64,
    total: u64,
    encoder: tiktoken_rs::CoreBPE,
    /// Set once the budget was blown or the inner stream ended.
    finished: bool,
}

impl<S> TokenCountStream<S> {
    fn new(inner: S, max_tokens: u64) -> Self {
        Self {
            inner,
            max_tokens,
            total: 0,
            // Embedded BPE data — loading it is effectively infallible.
            encoder: tiktoken_rs::o200k_base()
                .expect("o200k_base BPE data is embedded and always loads"),
            finished: false,
        }
    }
}

impl<S, I> Stream for TokenCountStream<S>
where
    S: Stream<Item = Result<I, Error>> + Unpin,
    I: serde::Serialize,
{
    type Item = Result<I, Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        if this.finished {
            return Poll::Ready(None);
        }
        match Pin::new(&mut this.inner).poll_next(cx) {
            Poll::Ready(Some(Ok(item))) => {
                // Tally before yielding: serialize, tokenize, accumulate.
                let json = match serde_json::to_string(&item) {
                    Ok(s) => s,
                    Err(e) => {
                        this.finished = true;
                        return Poll::Ready(Some(Err(Error::InlineJson(e))));
                    }
                };
                this.total += this.encoder.encode_with_special_tokens(&json).len() as u64;
                if this.total > this.max_tokens {
                    this.finished = true;
                    return Poll::Ready(Some(Err(Error::TokenBudgetExceeded {
                        limit: this.max_tokens,
                        actual: this.total,
                    })));
                }
                Poll::Ready(Some(Ok(item)))
            }
            Poll::Ready(Some(Err(e))) => Poll::Ready(Some(Err(e))),
            Poll::Ready(None) => {
                this.finished = true;
                Poll::Ready(None)
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

/// Stream adapter: a single whole-stream deadline.
///
/// The `tokio::time::sleep` is created on the FIRST `poll_next`, so the
/// deadline anchors at first poll, not at construction. Every poll
/// races the deadline against the inner stream; once it elapses — even
/// mid-wait on a slow item — the adapter yields one final
/// `Err(Error::Timeout)` and then ends. The stream can never outlive
/// `timeout_seconds`.
struct TimeoutStream<S> {
    inner: S,
    timeout_seconds: u64,
    /// Created lazily on first poll so the deadline anchors there.
    deadline: Option<Pin<Box<tokio::time::Sleep>>>,
    /// Set once the deadline fired or the inner stream ended.
    finished: bool,
}

impl<S> TimeoutStream<S> {
    fn new(inner: S, timeout_seconds: u64) -> Self {
        Self {
            inner,
            timeout_seconds,
            deadline: None,
            finished: false,
        }
    }
}

impl<S, I> Stream for TimeoutStream<S>
where
    S: Stream<Item = Result<I, Error>> + Unpin,
{
    type Item = Result<I, Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        if this.finished {
            return Poll::Ready(None);
        }
        let secs = this.timeout_seconds;
        let deadline = this.deadline.get_or_insert_with(|| {
            Box::pin(tokio::time::sleep(std::time::Duration::from_secs(secs)))
        });
        // Deadline first: it's a hard cap, so it wins ties.
        if deadline.as_mut().poll(cx).is_ready() {
            this.finished = true;
            return Poll::Ready(Some(Err(Error::Timeout {
                timeout_seconds: secs,
            })));
        }
        match Pin::new(&mut this.inner).poll_next(cx) {
            Poll::Ready(Some(item)) => Poll::Ready(Some(item)),
            Poll::Ready(None) => {
                this.finished = true;
                Poll::Ready(None)
            }
            Poll::Pending => Poll::Pending,
        }
    }
}
