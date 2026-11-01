//! Files to and from an entity, on this machine's disk: a download
//! saved where the person says, an upload read from what they dropped.
//!
//! Both run here, in Rust, over the page's own connection
//! ([`TauriProxy::link`]): the bytes go between the disk and the wire
//! in pieces of at most [`CHUNK_SIZE`](diverge_sdk::CHUNK_SIZE), one
//! frame ahead of whichever side is slower (see
//! [`credit`](diverge_sdk::wire::credit)), and never cross into the
//! page or sit whole in memory.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::time::{Duration, Instant};

use bytes::Bytes;
use diverge_sdk::daemon::endpoints::{agents, dependency_tools, standalone_tools, volumes};
use diverge_sdk::daemon::reference;
use diverge_sdk::shared::error::Error as Refusal;
use diverge_sdk::tauri_proxy::TauriProxy;
use diverge_sdk::wire::client::handle::Handle;
use futures_util::{Stream, StreamExt as _, stream};
use serde::{Deserialize, Serialize};
use tauri::ipc::Channel;
use tauri::{AppHandle, Manager as _, State};
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
use tokio::sync::{Mutex, oneshot, watch};

/// How often progress is told to the page, at most.
const TELL: Duration = Duration::from_millis(100);

/// Whose files: the family's own reference, as the page's model names
/// it — `{"agent":…}`, `{"tool":…}`, `{"dependency":…}`, `{"volume":…}`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Target {
    Agent(reference::Agent),
    Tool(reference::Tool),
    Dependency(reference::Dependency),
    Volume(reference::Volume),
}

/// How a transfer ended, as the page reads it.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Outcome {
    /// Every byte moved.
    Done { bytes: u64 },
    /// The person closed the dialog, or pressed ✕.
    Cancelled,
    /// Nothing is at the path, or the entity is not running (a
    /// dependency tool is never started for this).
    NotFound,
    /// The account may not.
    Forbidden,
    /// A running container has the volume, or another transfer is on it.
    Held,
    /// The folder a directory would be saved as is there already.
    Exists { path: String },
    /// Anything else, in words.
    Failed { why: String },
}

/// The transfers running, by the page's id for each: how each is
/// cancelled.
#[derive(Default)]
pub struct Transfers(Mutex<HashMap<u32, oneshot::Sender<()>>>);

impl Transfers {
    /// A transfer begins: the receiver its cancel arrives on.
    async fn start(&self, id: u32) -> oneshot::Receiver<()> {
        let (cancel, cancelled) = oneshot::channel();
        self.0.lock().await.insert(id, cancel);
        cancelled
    }

    /// A transfer is over: nothing more cancels it.
    async fn finish(&self, id: u32) {
        self.0.lock().await.remove(&id);
    }
}

/// Stop the transfer with this id. A transfer that is over is nothing.
#[tauri::command]
pub async fn files_cancel(transfers: State<'_, Transfers>, id: u32) -> Result<(), String> {
    if let Some(cancel) = transfers.0.lock().await.remove(&id) {
        let _ = cancel.send(());
    }
    Ok(())
}

/// Download what is at `path` in `files`.
///
/// The rules:
///
/// 1. A file is saved where the system's save dialog says, its name
///    offered; a directory, under the folder the folder dialog picks,
///    as a folder of its own name, which must not be there already.
///    Closing the dialog is [`Outcome::Cancelled`].
/// 2. Every chunk is written as it arrives, one file open at a time (a
///    file's chunks come together); every path the daemon names is
///    checked to be names, nothing above the folder.
/// 3. The bytes so far are told on `progress`, at most every [`TELL`].
/// 4. A file that did not finish is removed; a directory keeps what
///    landed.
/// 5. Cancelling drops the stream, which tells the daemon (see
///    [`credit`](diverge_sdk::wire::credit)).
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn files_download(
    app: AppHandle,
    proxy: State<'_, TauriProxy>,
    transfers: State<'_, Transfers>,
    id: u32,
    files: Target,
    path: Vec<String>,
    directory: bool,
    progress: Channel<u64>,
) -> Result<Outcome, String> {
    let name = path.last().cloned().unwrap_or_else(|| "files".to_string());
    // Rule 1.
    let dialog = rfd::AsyncFileDialog::new();
    // Over the window, as its own dialog.
    let dialog = match app.get_webview_window("main") {
        Some(window) => dialog.set_parent(&window),
        None => dialog,
    };
    let picked = if directory { dialog.pick_folder().await } else { dialog.set_file_name(&name).save_file().await };
    let Some(picked) = picked else {
        return Ok(Outcome::Cancelled);
    };
    let picked = picked.path().to_path_buf();
    let base = if directory {
        let base = picked.join(&name);
        if tokio::fs::metadata(&base).await.is_ok() {
            return Ok(Outcome::Exists { path: base.display().to_string() });
        }
        base
    } else {
        picked
    };
    let handle = proxy.link().handle().await.map_err(|refusal| format!("{refusal:?}"))?;
    // Rule 5.
    let cancelled = transfers.start(id).await;
    let outcome = tokio::select! {
        outcome = download(&handle, &files, path, &base, directory, &progress) => outcome,
        _ = cancelled => Outcome::Cancelled,
    };
    transfers.finish(id).await;
    // Rule 4: a file cut short is not left looking whole.
    if !directory && !matches!(outcome, Outcome::Done { .. }) {
        let _ = tokio::fs::remove_file(&base).await;
    }
    Ok(outcome)
}

/// What a download's stream says, the four families' items as one.
enum Got {
    Chunk(Vec<String>, Bytes),
    End(Outcome),
}

type Chunks = Pin<Box<dyn Stream<Item = Got> + Send>>;

/// Open the family's download.
async fn chunks(handle: &Handle, files: &Target, path: Vec<String>) -> Result<Chunks, String> {
    let failed = |error: &dyn std::fmt::Display| Got::End(Outcome::Failed { why: error.to_string() });
    Ok(match files {
        Target::Agent(agent) => {
            use agents::download::client::{execute, request};
            let opened = execute::execute(handle, &request::Frame { agent: agent.clone(), path, relay: None }).await.map_err(|error| error.to_string())?;
            Box::pin(opened.map(move |item| match item {
                Ok(execute::Item::Chunk { path, body }) => Got::Chunk(path, body),
                Ok(execute::Item::NotFound) => Got::End(Outcome::NotFound),
                Ok(execute::Item::Forbidden) => Got::End(Outcome::Forbidden),
                Ok(execute::Item::Error(error)) => failed(&error_words(&error)),
                Err(error) => failed(&error),
            }))
        }
        Target::Tool(tool) => {
            use standalone_tools::download::client::{execute, request};
            let opened = execute::execute(handle, &request::Frame { tool: tool.clone(), path, relay: None }).await.map_err(|error| error.to_string())?;
            Box::pin(opened.map(move |item| match item {
                Ok(execute::Item::Chunk { path, body }) => Got::Chunk(path, body),
                Ok(execute::Item::NotFound) => Got::End(Outcome::NotFound),
                Ok(execute::Item::Forbidden) => Got::End(Outcome::Forbidden),
                Ok(execute::Item::Error(error)) => failed(&error_words(&error)),
                Err(error) => failed(&error),
            }))
        }
        Target::Dependency(dependency) => {
            use dependency_tools::download::client::{execute, request};
            let opened = execute::execute(handle, &request::Frame { dependency: dependency.clone(), path, relay: None }).await.map_err(|error| error.to_string())?;
            Box::pin(opened.map(move |item| match item {
                Ok(execute::Item::Chunk { path, body }) => Got::Chunk(path, body),
                Ok(execute::Item::NotFound) => Got::End(Outcome::NotFound),
                Ok(execute::Item::Forbidden) => Got::End(Outcome::Forbidden),
                Ok(execute::Item::Error(error)) => failed(&error_words(&error)),
                Err(error) => failed(&error),
            }))
        }
        Target::Volume(volume) => {
            use volumes::download::client::{execute, request};
            let opened = execute::execute(handle, &request::Frame { volume: volume.clone(), path, relay: None }).await.map_err(|error| error.to_string())?;
            Box::pin(opened.map(move |item| match item {
                Ok(execute::Item::Chunk { path, body }) => Got::Chunk(path, body),
                Ok(execute::Item::NotFound) => Got::End(Outcome::NotFound),
                Ok(execute::Item::Held) => Got::End(Outcome::Held),
                Ok(execute::Item::Forbidden) => Got::End(Outcome::Forbidden),
                Ok(execute::Item::Error(error)) => failed(&error_words(&error)),
                Err(error) => failed(&error),
            }))
        }
    })
}

/// Rules 2 and 3.
async fn download(handle: &Handle, files: &Target, path: Vec<String>, base: &Path, directory: bool, progress: &Channel<u64>) -> Outcome {
    let mut chunks = match chunks(handle, files, path).await {
        Ok(chunks) => chunks,
        Err(why) => return Outcome::Failed { why },
    };
    if directory && let Err(error) = tokio::fs::create_dir_all(base).await {
        return Outcome::Failed { why: error.to_string() };
    }
    let mut open: Option<(Vec<String>, tokio::fs::File)> = None;
    let mut bytes = 0u64;
    let mut told = Instant::now();
    let outcome = loop {
        let (relative, body) = match chunks.next().await {
            None => break Outcome::Done { bytes },
            Some(Got::End(outcome)) => break outcome,
            Some(Got::Chunk(relative, body)) => (relative, body),
        };
        if open.as_ref().is_none_or(|(at, _)| *at != relative) {
            if let Some((_, mut file)) = open.take()
                && let Err(error) = file.flush().await
            {
                break Outcome::Failed { why: error.to_string() };
            }
            let Some(local) = local(base, &relative) else {
                break Outcome::Failed { why: format!("the daemon named a path that is not names: {}", relative.join("/")) };
            };
            if let Some(parent) = local.parent()
                && let Err(error) = tokio::fs::create_dir_all(parent).await
            {
                break Outcome::Failed { why: error.to_string() };
            }
            match tokio::fs::File::create(&local).await {
                Ok(file) => open = Some((relative, file)),
                Err(error) => break Outcome::Failed { why: format!("{}: {error}", local.display()) },
            }
        }
        if let Some((_, file)) = open.as_mut()
            && let Err(error) = file.write_all(&body).await
        {
            break Outcome::Failed { why: error.to_string() };
        }
        bytes += body.len() as u64;
        if told.elapsed() >= TELL {
            let _ = progress.send(bytes);
            told = Instant::now();
        }
    };
    if let Some((_, mut file)) = open.take() {
        let _ = file.flush().await;
    }
    let _ = progress.send(bytes);
    outcome
}

/// `relative` under `base`, every component a plain name; `None` for
/// one that is empty, `.`, `..`, or holds a separator or a drive.
fn local(base: &Path, relative: &[String]) -> Option<PathBuf> {
    let mut local = base.to_path_buf();
    for component in relative {
        let plain = !component.is_empty()
            && component != "."
            && component != ".."
            && !component.contains(['/', '\\', ':', '\0']);
        if !plain {
            return None;
        }
        local.push(component);
    }
    Some(local)
}

/// Upload `paths`, dropped on the page, into the directory `into` in
/// `files`.
///
/// The rules:
///
/// 1. Each dropped path is one upload, one after another: a file lands
///    at `into` and its own name, replacing a file there; a folder, as
///    a directory of its own name holding every file beneath it
///    (links not followed).
/// 2. Each file is opened when the daemon asks for it and read in
///    pieces as the wire takes them.
/// 3. The bytes so far are told on `progress`, at most every [`TELL`].
/// 4. The first upload that does not land ends the rest, and is the
///    outcome. Cancelling drops the exchange, which tells the daemon.
#[tauri::command]
pub async fn files_upload(
    proxy: State<'_, TauriProxy>,
    transfers: State<'_, Transfers>,
    id: u32,
    files: Target,
    into: Vec<String>,
    paths: Vec<String>,
    progress: Channel<u64>,
) -> Result<Outcome, String> {
    let handle = proxy.link().handle().await.map_err(|refusal| format!("{refusal:?}"))?;
    let (counter, counted) = watch::channel(0u64);
    // Rule 3: told from its own task, while the uploads run.
    let telling = tokio::spawn({
        let progress = progress.clone();
        let mut counted = counted.clone();
        async move {
            loop {
                let _ = progress.send(*counted.borrow_and_update());
                tokio::time::sleep(TELL).await;
                if counted.has_changed().is_err() {
                    return;
                }
            }
        }
    });
    let cancelled = transfers.start(id).await;
    let outcome = tokio::select! {
        outcome = uploads(&handle, &files, &into, &paths, &counter) => outcome,
        _ = cancelled => Outcome::Cancelled,
    };
    transfers.finish(id).await;
    drop(counter);
    telling.abort();
    let _ = progress.send(*counted.borrow());
    Ok(outcome)
}

/// Rules 1 and 4.
async fn uploads(handle: &Handle, files: &Target, into: &[String], paths: &[String], counter: &watch::Sender<u64>) -> Outcome {
    for dropped in paths {
        let dropped = PathBuf::from(dropped);
        let Some(name) = dropped.file_name().and_then(|name| name.to_str()).map(str::to_string) else {
            return Outcome::Failed { why: format!("{} has no name", dropped.display()) };
        };
        let meta = match tokio::fs::metadata(&dropped).await {
            Ok(meta) => meta,
            Err(error) => return Outcome::Failed { why: format!("{}: {error}", dropped.display()) },
        };
        let mut path = into.to_vec();
        path.push(name);
        let named = if meta.is_dir() {
            match walk(&dropped).await {
                Ok(named) if named.is_empty() => return Outcome::Failed { why: format!("{} holds no file", dropped.display()) },
                Ok(named) => Some(named),
                Err(why) => return Outcome::Failed { why },
            }
        } else {
            None
        };
        let outcome = upload(handle, files, path, named, dropped, counter).await;
        if !matches!(outcome, Outcome::Done { .. }) {
            return outcome;
        }
    }
    Outcome::Done { bytes: *counter.borrow() }
}

/// What an upload's content is: a file, opened when asked for and read
/// in pieces as they are pulled.
type Content = Pin<Box<dyn Stream<Item = Result<Bytes, Refusal>> + Send>>;

/// One upload, through the family's own endpoint: `named` the files of
/// a directory, `None` for a file.
async fn upload(handle: &Handle, files: &Target, path: Vec<String>, named: Option<Vec<String>>, from: PathBuf, counter: &watch::Sender<u64>) -> Outcome {
    let counter = counter.clone();
    // Rule 2: the file the daemon asks for, under what was dropped.
    let source = move |asked: Option<String>| -> Option<Content> {
        let local = match asked {
            None => from.clone(),
            Some(asked) => local(&from, &asked.split('/').map(str::to_string).collect::<Vec<_>>())?,
        };
        Some(read(local, counter.clone()))
    };
    let failed = |error: &dyn std::fmt::Display| Outcome::Failed { why: error.to_string() };
    match files {
        Target::Agent(agent) => {
            use agents::upload::{client::execute, client::request, server::response};
            let request = match named {
                None => request::Frame::File { agent: agent.clone(), path, relay: None },
                Some(files) => request::Frame::Directory { agent: agent.clone(), path, files, relay: None },
            };
            match execute::execute(handle, &request, source).await {
                Ok(response::Frame::Uploaded) => Outcome::Done { bytes: 0 },
                Ok(response::Frame::NotFound) => Outcome::NotFound,
                Ok(response::Frame::Forbidden) => Outcome::Forbidden,
                Ok(response::Frame::Error(error)) => failed(&error_words(&error)),
                Err(error) => failed(&error),
            }
        }
        Target::Tool(tool) => {
            use standalone_tools::upload::{client::execute, client::request, server::response};
            let request = match named {
                None => request::Frame::File { tool: tool.clone(), path, relay: None },
                Some(files) => request::Frame::Directory { tool: tool.clone(), path, files, relay: None },
            };
            match execute::execute(handle, &request, source).await {
                Ok(response::Frame::Uploaded) => Outcome::Done { bytes: 0 },
                Ok(response::Frame::NotFound) => Outcome::NotFound,
                Ok(response::Frame::Forbidden) => Outcome::Forbidden,
                Ok(response::Frame::Error(error)) => failed(&error_words(&error)),
                Err(error) => failed(&error),
            }
        }
        Target::Dependency(dependency) => {
            use dependency_tools::upload::{client::execute, client::request, server::response};
            let request = match named {
                None => request::Frame::File { dependency: dependency.clone(), path, relay: None },
                Some(files) => request::Frame::Directory { dependency: dependency.clone(), path, files, relay: None },
            };
            match execute::execute(handle, &request, source).await {
                Ok(response::Frame::Uploaded) => Outcome::Done { bytes: 0 },
                Ok(response::Frame::NotFound) => Outcome::NotFound,
                Ok(response::Frame::Forbidden) => Outcome::Forbidden,
                Ok(response::Frame::Error(error)) => failed(&error_words(&error)),
                Err(error) => failed(&error),
            }
        }
        Target::Volume(volume) => {
            use volumes::upload::{client::execute, client::request, server::response};
            let request = match named {
                None => request::Frame::File { volume: volume.clone(), path, relay: None },
                Some(files) => request::Frame::Directory { volume: volume.clone(), path, files, relay: None },
            };
            match execute::execute(handle, &request, source).await {
                Ok(response::Frame::Uploaded) => Outcome::Done { bytes: 0 },
                Ok(response::Frame::NotFound) => Outcome::NotFound,
                Ok(response::Frame::Held) => Outcome::Held,
                Ok(response::Frame::Forbidden) => Outcome::Forbidden,
                Ok(response::Frame::Error(error)) => failed(&error_words(&error)),
                Err(error) => failed(&error),
            }
        }
    }
}

/// Where a file's reading stands.
enum Reading {
    /// Not yet opened.
    Start(PathBuf),
    /// Open.
    Open(tokio::fs::File, PathBuf),
    /// Read to its end, or failed.
    Done,
}

/// The file at `local`, in pieces of at most `CHUNK_SIZE` as they are
/// pulled, each counted on `counter`; a file that cannot be read is
/// one piece's refusal, in the read's words, and the end.
fn read(local: PathBuf, counter: watch::Sender<u64>) -> Content {
    Box::pin(stream::unfold((Reading::Start(local), counter), |(reading, counter)| async move {
        let (mut file, local) = match reading {
            Reading::Done => return None,
            Reading::Open(file, local) => (file, local),
            Reading::Start(local) => match tokio::fs::File::open(&local).await {
                Ok(file) => (file, local),
                Err(error) => return Some((Err(refusal(&local, &error)), (Reading::Done, counter))),
            },
        };
        let mut buffer = vec![0u8; diverge_sdk::CHUNK_SIZE];
        match file.read(&mut buffer).await {
            Ok(0) => None,
            Ok(read) => {
                buffer.truncate(read);
                counter.send_modify(|bytes| *bytes += read as u64);
                Some((Ok(Bytes::from(buffer)), (Reading::Open(file, local), counter)))
            }
            Err(error) => Some((Err(refusal(&local, &error)), (Reading::Done, counter))),
        }
    }))
}

/// A read that failed, as the content's refusal.
fn refusal(local: &Path, error: &std::io::Error) -> Refusal {
    Refusal(serde_json::json!({
        "kind": "read",
        "path": local.display().to_string(),
        "error": error.to_string(),
    }))
}

/// The daemon's error, in words: its JSON as it said it.
fn error_words(error: &Refusal) -> String {
    error.0.to_string()
}

/// Every file under `root`, as `/`-joined paths from it; links are
/// not followed, and a name that is not text is the error.
async fn walk(root: &Path) -> Result<Vec<String>, String> {
    let mut named = Vec::new();
    let mut pending = vec![(root.to_path_buf(), String::new())];
    while let Some((dir, prefix)) = pending.pop() {
        let mut entries = tokio::fs::read_dir(&dir).await.map_err(|error| format!("{}: {error}", dir.display()))?;
        while let Some(entry) = entries.next_entry().await.map_err(|error| format!("{}: {error}", dir.display()))? {
            let kind = entry.file_type().await.map_err(|error| error.to_string())?;
            let name = entry.file_name().into_string().map_err(|name| format!("{} is not a name in text", name.to_string_lossy()))?;
            let relative = if prefix.is_empty() { name } else { format!("{prefix}/{name}") };
            if kind.is_dir() {
                pending.push((entry.path(), relative));
            } else if kind.is_file() {
                named.push(relative);
            }
        }
    }
    named.sort();
    Ok(named)
}
