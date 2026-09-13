# Report 16: both halves complete — the caller's answerers, the provider's handles, FUSE directories, and the tool path

## 1. Scope and method

This report records the state of the provider protocol and its
containers as of commit `2f9550571` (2026-09-10), stated as a
differential against the state report 15 recorded at commit
`47b195df2` (2026-09-09). It was prepared from the complete
`git diff 47b195df2..2f9550571`, from `git log` over the same range,
and from the files as they stand at the later commit. It reports
final positions only. Designs adopted and superseded within the range
are not described, except where a settled position is explained by
the alternative it replaced.

The range comprises twenty commits and touches 281 files (11,366
insertions, 2,853 deletions), distributed as follows:

| area | files | insertions | deletions |
|---|---|---|---|
| `diverge-provider-sdk` | 244 | 9,106 | 1,837 |
| `diverge-container-proxy` | 19 | 1,784 | 508 |
| `diverge-agentic-loop-eliza` | 17 | 466 | 508 |
| `diverge-agentic-loop-hermes` | 1 | 10 | 0 |

No other directory is touched. `diverge-agentic-loop-cc`,
`-codex`, `-openrouter`, `-python`, `diverge-container-proxy-sdk`,
`diverge-provider-web`, the workspace `Cargo.toml`, `Cargo.lock` and
`version.sh` are unchanged in the range. Three of the twenty commits
amend `STATUS_REPORT_15.md` itself; the amendments are recorded in
section 10.

Verification status, stated once and applicable throughout: the
provider SDK passes `cargo check` and `cargo doc --no-deps` with zero
warnings under each of its four feature combinations (`server,client`,
`server`, `client`, none); the proxy crate passes both on the host and
for `x86_64-unknown-linux-gnu`. No container image has been built.
Nothing has been run: no proxy, no container, no database, no vault,
no MCP server, no provider. No crate in the workspace implements any
of the provider-side traits. Every statement below about behaviour is
a statement about what the code does as written.

The headline: as of this commit every one of the ten endpoints has
both a `client::execute` and a `server::handle`. The client half is
declared complete in `client/mod.rs`; the server half's dispatcher
serves every request it can decode. What the crate does not do is run
anything itself, by design — a provider supplies the container
runtime, the volume store, the content store, the image registry's
HTTP, and the socket.

## 2. FUSE mounts: a file or a directory, and a `stat`

Report 15 introduced FUSE mounts as single files with two operations.
The position now is that a caller may mount a FILE or a DIRECTORY,
under one vocabulary of seven operations, and that a `stat` is its
own ask.

### 2.1 The request

`shared::containers::request::Container` carries two FUSE lists in
place of the former `fuse_mounts`:

```rust
#[serde(default, skip_serializing_if = "Vec::is_empty")]
pub fuse_file_mounts: Vec<FuseMount>,
#[serde(default, skip_serializing_if = "Vec::is_empty")]
pub fuse_directory_mounts: Vec<FuseMount>,
```

`FuseMount { container_path: Vec<String>, id: String, readonly: bool
}` is unchanged in shape; which kind a mount is, the list says. The
documented invariants: no component of a path is empty, `.` or `..`;
a FUSE file's path may lie inside a directory another mount provides
("a file over a directory a volume brought is the ordinary case") but
may not equal another mount's path; no other mount may lie inside a
FUSE directory mount; the root is refused; two mounts on one request,
on either list, may not share an id; a read-only mount refuses every
mutation inside the container and the provider never sends one for
its id.

The ruling that separates the two kinds is stated on the request, in
the vocabulary, and in the proxy, in the same terms: a file mount's
mount point is the file itself, and the kernel refuses to unlink or
rename a mount point, so a file mount can be read and overwritten in
place but never deleted, moved, or replaced by a rename. A program
that saves by writing a temporary beside its file and renaming over
it fails at the rename, because that rename is an operation on the
directory around the mount, which is not the proxy's. Such a program
is given a directory mount, where the whole tree is the caller's and
only the root is fixed.

### 2.2 The vocabulary `shared::containers::fuse`

Seven operations, each one ask and one one-message answer. Every ask
leads with `[id_len: u16 BE][id…]`; a path is `/`-separated UTF-8
relative to the mount root, no leading slash, EMPTY for a file mount
and for a directory mount's root; where a path ends the payload it
carries no prefix, where something follows it it does.

| ask | payload after the id | answer |
|---|---|---|
| `stat` | `[path…]` | `0` `[kind: u8][size: u64 BE]`, `1` missing, `2` error |
| `read` | `[path…]` | `0` the bytes, `1` missing, `2` error |
| `write` | `[path_len: u16 BE][path…][bytes…]` | `Ack`: `0` ok, `1` error |
| `list` | `[path…]` | `0` `[count: u32 BE]` then entries `[kind: u8][name_len: u16 BE][name…]`, `1` missing, `2` error |
| `remove` | `[path…]` | `Ack` |
| `rename` | `[from_len: u16 BE][from…][to…]` | `Ack` |
| `mkdir` | `[path…]` | `Ack` |

Types, as named in the module:

- `Target<'a> { id: &'a str, path: &'a str }` is the whole ask for
  `stat`, `read`, `list`, `remove` and `mkdir`; each of those
  `request::Request` is a type alias to it.
- `write::request::Request<'a> { id, path, bytes: &'a [u8] }` and
  `rename::request::Request<'a> { id, from, to }` carry the two asks
  that need a prefixed first path.
- `Kind { File = 0, Directory = 1 }`, with crate-private `byte()` and
  `from_byte()` shared by entries and stats.
- `Entry<'a> { name: &'a str, kind: Kind }` — a listing carries names
  and kinds only; an entry's size is a `stat` of its own.
- `stat::Stat { kind: Kind, size: u64 }` — nine fixed bytes; size `0`
  for a directory.
- `ack::Frame<'a> { Ok, Error(&'a str) }` answers `write`, `remove`,
  `rename` and `mkdir`; each of those `response::Frame` is an alias
  to it.
- `read::response::Frame<'a> { Present(&'a [u8]), Missing, Error(&'a
  str) }`; `stat::response::Frame<'a> { Present(Stat), Missing,
  Error(&'a str) }`; `list::response::Frame<'a> { Entries(Vec<Entry>),
  Missing, Error(&'a str) }`, whose count saturates at `u32::MAX`.
- Errors: `RequestError { Truncated, IdUtf8, PathUtf8 }`,
  `RequestEncodeError { IdLength(usize), PathLength(usize) }`,
  `ResponseError { Empty, UnknownKind(u8), MessageUtf8, Truncated,
  NameUtf8 }`, `ResponseEncodeError { NameLength(usize) }`.

The stated reason for `stat`: the kernel asks for an entry's
attributes far more often than for its bytes — on every `stat(2)`,
before most opens, once per component of every path it resolves — and
before this range the proxy answered each with a whole `read` on a
file mount and a whole parent `list` on a directory mount. A `stat`
answers with nine bytes; a file's bytes cross the wire only when a
program opens it. `Missing` from `stat` on a file mount is the same
absence `read` reports: nothing held yet, the file reads as empty.

Rules retained from report 15 and extended to the seven: a file is
one message either way (reads are not paged, writes are not chunked;
a mount is for credential- and configuration-sized files); a read-only
mount never sees a mutation; nothing retries, because a `write` or a
`rename` re-sent might undo what the caller did in between; the id is
the caller's and opaque.

### 2.3 The provider's channels toward the caller

Both run families' `server::channel_request::Frame` carry seven FUSE
variants at tags `18` FuseRead, `19` FuseWrite, `20` FuseList, `21`
FuseRemove, `22` FuseRename, `23` FuseMkdir, `24` FuseStat, each
wrapping the shared request type — twenty-five variants in all, and
the same twenty-five in both families, six the provider's own and
nineteen relayed. `FrameEncodeError` and `FrameError` each carry a
`Fuse(..)` variant. Both families' `client::channel_response` trees
have seven `fuse_*` modules, each a type alias to the shared answer;
the two trees are byte-identical.

### 2.4 The proxy wire

`container_proxy::requests::request::Request` carries the seven at
kinds `12` FuseRead through `17` FuseMkdir and `18` FuseStat, answered
on `/fuse/read/{channel}` and its six siblings. Each op under
`container_proxy::fuse` re-exports the shared module and, behind the
`server` feature, an executor `execute(client, channel, response:
Option<&response::Frame<'_>>) -> Result<(), ExecuteError>` — one
message when `Some`, then the clean close; `None` the wire's refusal.
`list`'s error enum alone carries `Encode(ResponseEncodeError)`, since
a listing can overflow a name's prefix; the other six carry `Open` and
`Socket` only.

The mounts environment `DIVERGE_CONTAINER_PROXY_FILESYSTEM_MOUNTS`
now holds `Mounts { files: Vec<Mount>, directories: Vec<Mount> }`,
`Mount { path: Vec<String>, id: String, readonly: bool }`, either list
absent; a value that does not parse is an error the proxy refuses to
start over.

### 2.5 The proxy crate's filesystems

`diverge-container-proxy/src/filesystem/mount.rs` is replaced by a
directory of five files. `mount(requests, handle, mount, kind)` takes
a `Kind::{File, Directory}`; for a file it creates the mount point as
a regular file (`0600`, or `0400` read-only) and mounts
`MountedFile`; for a directory it creates it (`0700` / `0500`) and
mounts `MountedDirectory`; both with `fuser` options `FSName(
"diverge-fuse")`, `DefaultPermissions`, `NoAtime`, `RO` or `RW`, one
thread, every parent directory made first. Every ask is the runtime
handle's `block_on` from the FUSE thread. `main.rs` mounts the files
then the directories before anything listens; a mount that cannot be
made ends the proxy as an unbindable port does.

`asks.rs` holds one `Asks` per mount — the id, the read-only flag,
and the seven asks. Every mutation passes `EROFS` for a read-only
mount before any ask is made; every transport failure and every error
the caller answers is `EIO` to the program, since what the caller
refused is not the program's to know. `handles.rs` is the per-handle
buffer table both filesystems share: `open`, `size`, `read`, `write`,
`resize`, `flush` (the store runs outside the lock), `release`,
`retarget` (a rename re-keys every handle under the moved entry).

The file mount (`file.rs`): one inode, the root, a regular file.
`getattr` with no handle asks `stat("")` — `File(size)` is the size,
nothing held is `0`, a directory answer is the caller's mistake and
`EIO`; `open` reads the bytes into a handle's buffer, or an empty one
under `O_TRUNC`; `read` and `write` work the buffer; `flush`, `fsync`
and `release` of a dirty buffer store it whole, and `release` always
answers ok because its error reaches nobody; `truncate(2)` with no
handle reads, resizes and writes; `lookup` is `ENOTDIR`. Attribute
TTL is zero: every `stat(2)` is the caller's current answer.

The directory mount (`directory.rs`): an inode table keyed by path,
numbers handed out as the kernel looks paths up, freed when it
forgets, re-keyed under a rename so a handle taken before a move still
names its entry. The root is a directory the proxy answers itself;
every other `lookup` and `getattr` is one `stat` of the entry;
`readdir` is one `list`, with `.` and `..` synthesized; `open` is a
`read` into a buffer and a changed close a `write` of it; `create`
writes an empty file with the caller at once, so `O_EXCL` and a
`stat` before the first close see it; `mkdir`, `unlink` and `rmdir`
are the caller's `mkdir` and `remove`; `rename` honours
`RENAME_NOREPLACE` by a `stat` first, answers `RENAME_EXCHANGE` with
`ENOTSUP`, and re-keys inodes and handles on success; `mknod` is
`EPERM`; times, mode and owner are accepted and change nothing.

### 2.6 What the containers say

`diverge-agentic-loop-hermes/HARNESS.md` gains one rule: the
alternative to its vault refresh cycle is a FUSE directory mount of
`$HERMES_HOME`, or of the directory holding its auth store, served
live by the caller. A directory, not a file, because Hermes saves its
auth store by a temporary and an atomic `os.replace`
(`hermes_cli/auth.py`, `_save_auth_store`), which a single-file mount
cannot take.

`diverge-agentic-loop-eliza/HARNESS.md` states the same for
`@elizaos/plugin-codex-cli`, which refreshes Codex's `auth.json` by
temporary-and-rename: its token cache lives in a FUSE directory mount
with `CODEX_AUTH_PATH` naming the file inside it.

The cc container needs no change: Claude Code rewrites its
credentials file in place, which a file mount permits. The codex
container is unchanged in this range; its position — a mounted
`auth.json` first, the vault's `OPENAI_API_KEY` second, the vault's
OAuth document never — predates the range and is restated in section
10.

## 3. The client half, complete

`client/mod.rs` now states: "Every endpoint has its executor: the five
`volumes`, `images::check` and `version` collapse into a call or a
stream; the three `containers` scopes hand back a handle that holds
the container's life. The client half is complete."

### 3.1 What a caller supplies: eight traits

One file each under `src/client/`, each `Send + Sync`, each method
returning `impl Future<Output = …> + Send`, none carrying an error type
of its own — an absence is the wire's empty finish, a refusal the frame
that says so. Streams are associated types, never boxed.

| trait | answers | surface |
|---|---|---|
| `OciStore` | the manifest and blobs of an image the caller holds | `type Blob: Stream<Item = Bytes> + Send + 'static`; `manifest(digest) -> Option<Manifest { media_type: String, body: Bytes }>`; `blob(digest) -> Option<Self::Blob>` |
| `ConnectionAuthorizer` | whether a connector may attach | `authorize(&Authorize) -> authorize::response::Frame` |
| `IdentityStore` | mounted content the provider does not hold | `type File: Stream<Item = Bytes>`; `type Directory: Stream<Item = (Vec<String>, Bytes)>`; `file(identity) -> Option<Self::File>`; `directory(identity) -> Option<Self::Directory>` |
| `PostgresDialer` | the container's database connections | `type Connection: Stream<Item = Bytes>`; `dial(connection_id: u32, from_container: UnboundedReceiver<Bytes>) -> Option<Self::Connection>` |
| `CommandRunner` | the commands the container asks run | `type Items: Stream<Item = Result<Bytes, Error>>`; `run(command: Bytes) -> Self::Items` |
| `Vault` | the container's secrets, with locks | `get -> Result<Option<Bytes>, String>`; `set`, `delete`, `lock(key, ttl: u32)`, `unlock -> Result<(), String>` |
| `McpServer` | the container's tool calls outward | `type Notifications: Stream<Item = Result<ServerNotification, ErrorData>>`; the four rmcp exchanges returning `Result<_, ErrorData>`; `notifications() -> Self::Notifications` |
| `FuseServer` | the files and directories mounted live | `stat -> Result<Option<Stat>, String>`; `read -> Result<Option<Bytes>, String>`; `write(id, path, Bytes)`, `remove`, `rename(id, from, to)`, `mkdir -> Result<(), String>`; `list -> Result<Option<Vec<Listed { name, kind }>>, String>` |

`Answerers<O, A, I, P, C, V, M, F>` bundles the eight behind `Arc`s
(`oci`, `authorizer`, `identities`, `postgres`, `commands`, `vault`,
`mcp`, `fuse`), with a hand-written `Clone` and no bounds on the
struct; the bounds are spelled where an executor takes it. A `Vault`
lock resolves when held, its TTL runs from the grant, the holder is
the container and not a connection, a re-lock refreshes, expiry
releases silently, a non-holder's unlock and a TTL of `0` are errors.

### 3.2 The shared machinery `endpoints::containers::client`

Behind the `client` feature, written once for the three scopes:

- `Ask`, the twenty-five asks a run scope's provider makes, owned,
  with a `From` impl from each run family's server frame.
- `Answered`, a marker trait per client-opened answer — `type Item`,
  `type Error`, `type Refusal`, `fn decode(Bytes) -> Result<Result<
  Item, Refusal>, Error>` — with implementors for `Postgres`,
  `RunLoop`, `AgentSchema`, `Enqueue`, `Dequeue`, the five MCP
  answers, and, beside each family's `execute`, `Filetree`, `Read`
  and `WritePath` in that family's own envelope. `Enqueue` and
  `Dequeue` have no refusal: their `Error` variant is part of the
  item, because nothing refuses an enqueue, it is only fated.
- `ChannelStream<A>`, a client-opened channel read as a stream of
  `A::Item`: zero or more `Ok`, then the end or exactly one terminal
  `Err` — `ChannelStreamError::{Closed, Frame, Misrouted, Response(
  A::Error), Refused(A::Refusal)}`. `None` is the provider's finish;
  `Closed` is the connection going away, "one says the exchange is
  over, the other says nothing at all." `FusedStream` reports the end
  honestly. The marker is `PhantomData<fn() -> A>` so the stream is
  `Unpin`, `Send` and `Sync` whatever the marker.
- `unary::<A>`, the one-frame reader: the first frame or
  `UnaryError::Unanswered` when the finish comes first — the wire's
  standing could-not-serve — plus `Request`, `Send`, `Closed`,
  `Frame`, `Misrouted`, `Response`, `Refused`.
- `Scoped`, the running scope every `ExecuteHandle` wraps behind an
  `Arc`: the handle and number, the pending `Writes`, and the main
  stream's end behind a mutex, answered the same way to every later
  `wait` — `WaitError::{Closed, Frame, Misrouted, Response(R),
  Provider(Error)}`.
- `Writes`, the content a caller's write holds between its two
  exchanges, by write id, taken back out when the request never goes
  out.
- `Encoders { postgres_half, write_body, write_error }`, the three
  frames a family encodes for the shared machinery, and `OpenError {
  Request, Send }` for what fails before the wire.
- `serve`, the loop over a scope's server-opened channel requests,
  spawning one task per ask; `answer/`, one file per exchange, each
  frames then the finish, chunked at `CHUNK_SIZE` where the answer is
  bytes, the empty finish where the answerer holds nothing, and no
  retry anywhere. The postgres answer opens this end's own half
  through `encoders.postgres_half` before forwarding either
  direction, so a half that cannot open is a dial declined. A `Write`
  ask for an id with nothing pending answers one `write_error` frame
  naming the id.

### 3.3 The three executors

`agents::run::client::execute` and `tools::run::client::execute`:

```rust
pub async fn execute<O, A, I, P, C, V, M, F>(
    handle: &Handle,
    request: &request::Frame,
    answerers: Answerers<O, A, I, P, C, V, M, F>,
) -> Result<(Id, ExecuteHandle), ExecuteError>
```

The request goes out; the first main-stream frame is read — `Id` is
success, `Error` is `ExecuteError::Provider`, a finish first is
`Unanswered`; then the serving task is spawned and the handle
returned. `ExecuteError::{Send, Request, Closed, Frame, Unanswered,
Misrouted, Response, Provider}`. Dropping the handle ends nothing.
`ExecuteHandle` is `Clone` over an `Arc<Scoped>` with `scope()`,
`wait()`, `stop()` (a bare channel; the answer is the scope's finish
on `wait`), `filetree() -> FiletreeStream`, `read(path) ->
ReadStream`, `write(path, content: impl Stream<Item = Result<Bytes,
E>>) -> Result<(), UnaryError<WritePath>>`; the agents family adds
`run_loop(prompt) -> RunLoopStream`, `agent_schema() -> Value`,
`enqueue(prompt) -> enqueue::response::Frame`, `dequeue() ->
dequeue::response::Frame`; the tools family adds `list_tools`,
`list_resources`, `call_tool`, `read_resource` returning the rmcp
results under `UnaryError<McpX>`, and `notifications() ->
McpNotificationsStream`.

`tools::connect::client::execute(handle, request) ->
Result<ExecuteHandle, ExecuteError>` takes no answerers and waits for
no frame: the main stream says nothing on success, so the request
goes out and the handle comes back, and a container that was not
there or a runner that said no arrives as the provider's error on
`wait`. `ExecuteError::{Send, Request}` only. Its serving task
answers the one ask a provider makes of a connector — the content of
its own writes — and drops anything else. Its handle has `disconnect`
in place of `stop`, "which takes nothing with it, since a connector
joined something it does not own," and the same filetree, read, write
and five MCP methods as `tools::run`.

### 3.4 Removals and renames

- `endpoints/containers/agents/agent/` is deleted (316 lines): the
  reference `Agent { Codex, Python }` definitions held "for reference,
  and not on the wire." Each image owns its agent type and states it
  through `agent_schema`; the `agent` field's doc on the run request
  says so.
- `Container.file_mounts` and `directory_mounts` are renamed
  `identity_file_mounts` and `identity_directory_mounts`, "so the
  identity mounts say what they are beside the fuse lists"; the
  `IdentityMount`, `fetch_file` and `fetch_directory` docs follow.
- `client/handle.rs` loses two `#[allow(dead_code)]` and their
  "Nothing is written yet" notes; two stale `laboratories::connect`
  links in the volumes watch executor now name
  `containers::tools::connect`.

## 4. `Container` is an address and a stop

`server::container::Container` is reduced to two methods:

```rust
pub trait Container: Send + Sync {
    fn address(&self) -> &str;
    fn stop(&self) -> impl Future<Output = ()> + Send;
}
```

`address` is the base URL a `ContainerClient` takes — `ws://host:port`
— naming the container's `OUTSIDE_PORT` however the runtime made it
reachable; a fact the deploy learned, so it cannot fail and need not
wait. Removed from the trait: the `Error` type, `mcp_serve`, the five
`mcp_*` asks, `agentic_loop`, `postgres_serve`, `command_serve`,
`filetree`, `read`, `write`, every stream, sink and responder type
they carried, and the `McpRequest`, `McpResponder` and `ContentError`
helpers. The stated reason: every one of them was the proxy's protocol
restated as a trait, to be implemented by dialling the proxy; the dial
is the whole of it, so the address is the whole of it.

`Deployment` loses `ports`. There is exactly one port and it is always
the same, the proxy's `OUTSIDE_PORT`, which a deployer makes reachable
on every container; the entrypoint's port is behind the proxy on the
loopback and is never published. `Deployment` gains
`identity_file_mounts` and `identity_directory_mounts`, the wire's
`IdentityMount`s, every identity held by the store by the time a
deploy is asked for. FUSE mounts are not on the deployment: the proxy
makes them from the environment, and all a deployer owes them is
`/dev/fuse` and the privilege every container gets.

`ContainerDeployer`'s contract is restated: deploying is also
injecting the proxy into the container and starting it; by the time a
deploy method returns `Ok`, the proxy accepts a connection at the
container's address. An entrypoint that never binds is still not
detectable here: it surfaces as an exchange the proxy cannot serve.
The trait's `Error` remains unbounded; the `Into<Error>` bound lives
on `server::handle`.

## 5. The tool container's MCP server, through the proxy

A tools scope's caller opens `McpListTools`, `McpListResources`,
`McpCallTool`, `McpReadResource` and `McpNotifications` into the tool
container's own MCP server. With the `Container` trait no longer
carrying those exchanges, the proxy carries them.

### 5.1 The proxy crate: `/tool/*`

Five paths on the server-facing listener, forwarded like `/agent/*`:
`/tool/list-tools`, `/tool/list-resources`, `/tool/call-tool`,
`/tool/read-resource`, `/tool/notifications`. The tool container's
server is Streamable HTTP on the loopback at `/mcp`, on the port
`agent::port()` reads — the one `PORT` rule (default `8080`) for both
kinds of entrypoint.

`src/tool/client.rs` holds one rmcp client, `Tool`: dialled on the
first `/tool/*` opening, not at startup; kept for the container's
life; re-dialled when its transport has closed. Its `Handler` answers
the server's ping and nothing else, and hands every notification to
a broadcast channel of 256. Each unary handler reads the first binary
message as the params, calls the peer, and answers one shared
`response::Frame` then the close: the server's own error travels
through as itself, code and all; a call the transport lost is an
internal error with the reason; a first message that is not binary,
or params that will not parse, is `invalid_params` — never a silent
close, since the caller asked in a vocabulary and is owed an answer in
it. `/tool/notifications` dials first, so an unreachable server
answers `Error` at once, then relays one frame per notification until
the server leaves; a subscriber that lagged loses the oldest and goes
on. `Cargo.toml`'s `rmcp` features gain `client` and
`transport-streamable-http-client-reqwest`; `AppState` gains
`tool: Arc<Tool>`.

### 5.2 The SDK: `container_proxy::tool`

Five modules, each re-exporting the shared `mcp::<op>` types and,
behind `server`, an executor: the four unary
`execute(client, &request::Request) -> Result<response::Frame,
ExecuteError>` hand the answering frame back whole, `Error` included,
because that variant is the server's answer and not a failure of the
asking — `ExecuteError::{Open, Encode, Frame, Unserved, Text, Socket,
Closed}`; `notifications::execute(client) -> Result<ExecuteStream,
ExecuteError>` yields `Result<ServerNotification, ExecuteStreamError>`
with `Refused(ErrorData)` ending it. The `container_proxy` path table
gains the two rows.

## 6. What a provider supplies: three more things

### 6.1 `ContentStore`

```rust
pub trait ContentStore: Send + Sync {
    type Error: Send + 'static;
    fn holds(&self, identity: &str) -> impl Future<Output = bool> + Send;
    fn store_file<S>(&self, identity: &str, content: S) -> impl Future<Output = Result<(), Self::Error>> + Send
    where S: Stream<Item = Bytes> + Send + 'static;
    fn store_directory<S>(&self, identity: &str, files: S) -> impl Future<Output = Result<(), Self::Error>> + Send
    where S: Stream<Item = (Vec<String>, Bytes)> + Send + 'static;
}
```

Where the content a caller mounts by identity is kept. A run handler
asks `holds` for every identity a request names and fetches only the
rest — a provider may hold any of them already, from an earlier run
of anyone's — all of it before the deploy. The store verifies: the
identity carries the size and the hash, and an implementation checks
them before an identity counts as held; the crate hashes nothing. A
stream that ended short is caught the same way.

### 6.2 `ImageRegistry` and `ImageSource`

```rust
pub trait ImageRegistry: Send + Sync {
    type Error: Send + 'static;
    fn address(&self) -> SocketAddr;
    fn serve(&self, repository: &str, source: ImageSource) -> impl Future<Output = Result<(), Self::Error>> + Send;
    fn release(&self, repository: &str) -> impl Future<Output = ()> + Send;
}
```

The provider's OCI registry, the read side of the Distribution API on
its loopback, that its runtime pulls a caller-held image from. A run
handler tells it, before the deploy, that a repository is to be served
from an `ImageSource`; the deployer then points the runtime at
`<address>/<repository>/<name>@<digest>`. For a digest the registry
does not hold, under a repository it is serving, it asks the source,
hashes what lands, keeps it only if the hash is the digest, and
answers — or `404`, which is what makes the deploy fail. Ranges are
served from the store, never asked of the caller. A repository is a
run: named by the handler, unique per run, released when the run
ends; what was stored stays stored. The registry's HTTP is the
provider's because the crate serves none, by standing design.

`server::image_source::ImageSource` is the crate's: `Clone`, holding
the run scope, with `manifest(digest) -> Option<Manifest { media_type:
String, body: Bytes }>` — one `OciManifest` channel, read to its
finish — and `blob(digest) -> Option<BlobStream>` — one `OciBlob`
channel, `None` on an empty finish or a caller that is gone, otherwise
the pieces in order ending at the finish, a channel closed short
ending the stream so the registry's hash misses the piece that never
came.

### 6.3 `Directory`

`server::directory::Directory` is every container the provider is
running, by id: one per provider, shared across connections, because
a connector names a container its runner may have started on another
socket. `mint()` is a v4 UUID — the id is a capability, unguessable,
never derived from anything a caller chose; `insert(id, scope,
address)` once a run's id is minted; `remove(id)` when the run ends,
which flips a `watch` every attached connector holds (sent with
`send_replace`, so a connector that looks later still reads the end);
`lookup(id) -> Option<Attached { scope: Arc<ScopeHandle>, address:
String, ended: watch::Receiver<bool> }>`. Nothing enumerates it.

### 6.4 The plumbing that changed for them

- `server::handle::handle<D, V, I, U, S, R>` takes, after the session,
  the authorization and the peer address: `deployer: Arc<D>`,
  `volume_manager: Arc<V>`, `image_checker: Arc<I>`, `content_store:
  Arc<S>`, `image_registry: Arc<R>`, `directory: Arc<Directory>`,
  with `S: ContentStore + 'static, S::Error: Into<Error>, R:
  ImageRegistry + 'static, R::Error: Into<Error>` added to the bounds.
  The three container arms spawn their handlers as the other seven
  do, cloning the `Arc`s they need.
- `ScopeHandle::send_response_finish` takes `&self` rather than
  consuming the handle. A container scope is held behind an `Arc` by
  every task that serves it, and a finish that needed the last
  reference would wait on all of them; the rule that a finish is once
  and last is now the handler's to keep, and it must not race a task
  inside `recv_channel_request`, which holds the inbox the finish
  closes. The seven existing handlers call it unchanged.
- Two crate-private helpers: `server::answer::answer(&Bytes) ->
  Option<Answer::{Frame(Bytes), Finish}>`, one frame off a
  server-opened channel read for what it is; and
  `server::answers::Answers`, a server-opened channel read as a
  `Stream<Item = Result<Bytes, Unfinished>>` — the payloads until the
  finish, or one `Err` for a receiver that closed without one —
  opened plain by `open` or peeked by `first`, which is `None` on an
  empty finish.
- `server/mod.rs` names the supplied things as six traits and one
  type, and states that every endpoint is handled.

## 7. The server handles

`endpoints::containers::server`, behind the `server` feature, is the
mirror of `endpoints::containers::client`: the provider's side of the
three scopes written once, and each `handle` wrapping it.

### 7.1 The machinery

- `family::Family`, one scope's frames named once: `type Request`
  (the client channel request the caller opens), `type Exchange` (the
  family's own exchanges past the shared five), `classify(Request) ->
  Opened<Exchange>`, `serve(run, channel, exchange)`, and the
  encoders — `error`, `write_ask`, `content`, `filetree`,
  `filetree_error`, `read_body`, `read_error`, `written`,
  `write_error` — each answering `None` for a frame that would not
  serialize, which is sent as nothing. `family::Runs: Family` adds
  `type Ask<'a>: Encode + From<Own<'a>>`, `relayed(Request<'a>) ->
  Option<Self::Ask<'a>>` (the container's ask as this family's frame;
  `None` for `Postgres`, which is re-asked under this end's own id)
  and `id(&Id)`. `Opened<E> { Stop, Filetree, Read(path), Write {
  write_id, path }, Postgres(connection_id), Exchange(E) }`.
- `own::Own<'a> { OciManifest(&str), OciBlob(&str),
  Authorize(Authorize), FetchFile(&str), FetchDirectory(&str),
  Postgres(u32) }`, the provider's own asks, spelled by each run
  family's `From`.
- `run::Run`: the scope behind an `Arc`, the `ContainerClient`, the
  `Pairs`, a `Notify` that the container is gone, and a `JoinSet` of
  the scope's tasks — `spawn`, `shutdown` (abort all, then join),
  `respond`, `finish`.
- `pairs::Pairs`: the database connections in flight, by the id this
  end minted — `open() -> (id, UnboundedSender<Bytes>)`, `take(id)`,
  `forget(id)` — the queue the container's bytes wait in until the
  caller's half opens.
- `encoded`, `render` (`proxy`, `refused`, `missing_content`,
  `content_stopped`, each a `{"kind": …}` JSON error).

Each run family's `handle/family.rs` implements `Family` and `Runs`
against its own types — `Agents` with `serve::agent::Exchange {
RunLoop(prompt), AgentSchema, Enqueue(prompt), Dequeue }`, `Tools`
with `serve::tool::Exchange { ListTools, ListResources, CallTool,
ReadResource, Notifications }` — and the `From<Own<'a>>` for its
server channel request frame. The connect family's `Connect`
implements `Family` only, with `tool::Exchange`, `Disconnect`
classified as `Stop`, and the connect scope's own write-content ask
and response envelopes.

### 7.2 `setup::prepare`: the order

`prepare::<R, D, S, G>(scope, client_identity, request, deployer,
store, registry) -> Result<Prepared { container, client, asks,
repository: Option<String> }, Error>`, in the only order that works:

1. `content::ensure`: for every identity file and directory mount,
   `store.holds`; the missing ones fetched concurrently on
   `FetchFile` / `FetchDirectory` channels and stored, an empty
   finish being `{"kind":"content","identity":…}` — because a deploy
   binds them and cannot wait for them.
2. The `Deployment` built: memory and disk as sent; volume `Mount`s
   stamped with the caller's identity; the identity mounts as sent;
   the environment holding `DIVERGE_CONTAINER_PROXY_FILESYSTEM_MOUNTS`
   rendered from the two FUSE lists and
   `DIVERGE_CONTAINER_PROXY_FILETREE_IGNORE` naming every mount's path,
   volume, identity and FUSE alike.
3. For `Image::Client`, a repository named by a fresh UUID and
   `registry.serve(repository, ImageSource)` — before the deploy,
   because the deploy is what pulls.
4. The deploy through the matching deployer method; a failure
   releases the repository and is the run's error.
5. `ContainerClient::new(container.address())` and
   `requests::execute` — the one `/requests` connection; a proxy that
   does not answer is a container that never came up, stopped and
   released.

### 7.3 `relay`: the container's asks

One task per ask off the `/requests` stream, dispatched by kind:

- MCP unary, vault, and the seven FUSE asks are the one-frame pattern
  (`relay::one::ask`): the ask carried to the caller as the family's
  frame, its channel read to the finish, the first payload decoded as
  the shared answer and handed to the matching proxy executor — or
  `None`, the proxy's refusal, when the caller finished with nothing
  or is gone.
- MCP notifications and commands stream: the caller's frames decoded
  and sent through the executor's handle, the caller's finish becoming
  the handle's finish; a caller that goes away leaves the path
  unfinished, which the proxy reads by its own rules.
- Postgres is the pair: a connection id minted and `Pairs::open`ed;
  this end's half opened toward the caller (`Postgres {
  connection_id }`, whose answers are what the database says); the
  proxy's `/postgres/{channel}` split into the driver's two
  directions; a task carries the database's answers into the socket
  and the caller's finish — or its empty finish at once, declining —
  closes it; the relay carries the driver's bytes into the pair's
  queue until the socket ends, then drops the sender and forgets an
  untaken queue.

The asks stream ending, cleanly or not, notifies the run that the
container is gone.

### 7.4 `serve`: the caller's channels

One loop races `recv_channel_request` against the run's `over`
signal. Each frame is one channel the caller opened: a request that
will not decode is finished with nothing; `Stop` ends the loop;
`Filetree`, `Read`, `Write` and `Postgres` go to the shared tasks; the
family's own to `Family::serve`. The loop returns `End::{Stopped,
Over, Gone}`.

- `filetree`: `tree::execute` → each frame as the family's `Filetree`
  response, `Refused` and other failures as its `Error`, then the
  finish.
- `read`: `read::execute` → each piece as `Body`, `Unserved` as the
  finish alone, `Refused` and other failures as `Error`, then the
  finish.
- `write`: the family's write-content ask opened for the caller's
  `write_id`; its answers, each decoded through `Family::content`,
  are the stream fed to `filesystem::write::execute` for the path; a
  receiver closed without a finish is a content error; the answer is
  `Written`, the content's own error, the proxy's refusal, or the
  provider's — and `Unserved` the finish alone.
- `postgres`: the caller's half — `Pairs::take(connection_id)`, the
  queue drained as this channel's responses until the driver's socket
  ends, then the finish; an id this end never minted or already taken
  is the finish with nothing.
- `agent`: `RunLoop` → `agent::run::execute`, chunks as `Chunk`, the
  container's `Refused` as `Error`; `AgentSchema`, `Enqueue`,
  `Dequeue` → one frame then the finish, an executor failure the
  finish alone.
- `tool`: the four unary `/tool/*` executors' frames sent whole,
  `notifications` relayed one frame each with the proxy's `Error`
  last.

### 7.5 `handler::run` and the two run handles

One function both run families share: `prepare`; for an agent
container, `agent::register::execute` with the request's `agent`
value, a refusal being the container's own error with the container
stopped and the repository released; the id minted, the container
entered in the `Directory`, the id sent; the relay spawned; the serve
loop; and teardown, the same for every ending — the directory entry
removed (which ends every connector), the container stopped, the
repository released, the tasks aborted and joined, the scope
finished. A failure before the id is `Error` then the finish, with
everything the caller may have opened meanwhile dropped unanswered.
An ending after the id is never an error: `Stop`, the container
leaving, and the caller going away all finish bare.

`agents::run::server::handle::handle(scope, request,
client_identity, deployer, store, registry, directory)` passes
`Some(request.agent)`; `tools::run`'s passes `None`.

### 7.6 The connect handle

`tools::connect::server::handle::handle(scope, request, address:
IpAddr, directory)`: the container found in the `Directory` by its id,
or `{"kind":"missing"}` then the finish; the runner asked on the RUN
scope with an `Authorize { address, authorization }` encoded as the
`tools::run` family's frame — the address this connection's peer,
attested, the authorization the connector's, asserted — its channel
read to the finish; `Denied`, an empty finish, or a runner that is
gone is `{"kind":"denied"}` then the finish; `Authorized` leaves the
main stream quiet. Then a `Run` over the run's address, a task that
watches the directory's `ended` signal and notifies `over`, the same
serve loop, and teardown: the tasks ended, the finish bare. Nothing
is stopped and nothing released, since the container is its runner's.
A connector's `Postgres` finds no pair and is finished with nothing.

### 7.7 Two decisions recorded

Teardown aborts a run's tasks rather than draining them: by then the
container is stopped or left, so what a task was doing cannot
complete, and a task waiting on the caller for an answer that may
never come must not hold the scope's finish hostage. What an aborted
task leaves is a channel the caller finishes on its own, or the
connection ends.

A tool container's MCP server is assumed at
`http://127.0.0.1:{PORT}/mcp` under the agent server's `PORT` rule.
Nothing in the tools family's request names a port.

## 8. The eliza container: model-provider plugins are a field

One commit rewrites the eliza harness's plugin model; a second adds
the FUSE directory rule of section 2.6.

### 8.1 The agent

`Agent` is five fields: `character`, `memory`, `generate_media:
Option<bool>`, `model_provider_plugins: Vec<Plugin>`, `plugins:
Vec<Plugin>`. `Plugin { package, settings, secrets }` is unchanged.
The former `provider`, `embedding` and `toolsets` fields and their
files are deleted; `toolsets.generate_media` became
`Agent::generate_media` (core's `GENERATE_MEDIA` action, unregistered
after `initialize()` when off); `toolsets.documents` became
`Memory::documents: Option<bool>` (core's native `enableDocuments`,
distinct from the `plugin-documents` package); coding tools, browser
and web search are plain `plugins` entries. There is no `image`
field: the image pre-installs packages and is never named in the
request.

The rule between the two lists: a plugin that registers a model
handler belongs under `model_provider_plugins` and nowhere else; the
first listed answers every model type it registers, the next is its
failover — Eliza's own priority order, `priority = n − index`, walked
down on a fallback-class error (a rate limit, a 5xx or 529, a timeout
or network failure; never a 401 or a 400). Absent is no model at all,
and such a run fails as Eliza's own failure. `ELIZA_BRAIN_PROVIDER` is
not rendered.

### 8.2 Enforced twice

In `node/entry.mjs`, the plugin array is the adapter
(`@elizaos/plugin-sql`, the one plugin the entry loads on its own,
pointed at the caller's database by `POSTGRES_URL`), the diverge
plugin, the model providers in the agent's order, the other plugins in
the agent's order. Statically, at import: a `model_provider_plugins`
entry whose `plugin.models` declares no handler is refused naming the
package and the list it belongs in, and a `plugins` entry declaring
any is refused likewise. Dynamically, after `initialize()`: every
registration `runtime.getModelRegistrations()` holds whose `provider`
is neither the adapter's name nor a listed provider's is refused,
naming the handler — so a plugin registering a model from its `init`
is caught too. Either is fatal before `ready`, the run's one `Err`, a
`500` with the message.

### 8.3 Nothing is pre-wired but the adapter

`settings.rs` renders exactly one setting of the harness's own,
`POSTGRES_URL`, then each listed plugin's `settings`, model providers
first, a later entry's same-named setting winning, then the vault's
values. `vault.rs` reads every plugin's `secrets` on either list and
implies no key of its own: `OPENAI_API_KEY` is plugin-openai's to be
given as a secret on its entry, as `TAVILY_API_KEY` is
plugin-web-search's. The image's pinned packages — `plugin-openai`,
`plugin-embeddings`, `plugin-coding-tools`, `plugin-browser`,
`plugin-documents`, `plugin-web-search`, `vault`, all
`2.0.3-beta.7` — are pre-installed and nothing more: `plugins.rs`
satisfies a bare spec, or one at the on-disk version, from the image's
copy without a `bun add`, and installs a spec naming another version;
the lineage then pins the version in use. The `/documents` directory
is no longer made in the image.

### 8.4 The lineage and the wire

`eliza_lineage` loses `embedding_dimensions`: which provider embeds,
and at what width, is the caller's list's, and Eliza pins the first
embedding provider that answers and re-embeds in the background when
the width changes; the non-fatal `embedding_dimensions` note is gone
with it. `Lineage::changed(&[Resolved])` and `save(pool,
Vec<Resolved>)` lose their dimensions argument; `plugins` records
both lists, model providers first. The `configure` line to the entry
carries `modelProviderPlugins` and `plugins` in place of `installed`
and `plugins`. `HARNESS.md` records all of the above and revises its
facts-to-establish list accordingly.

## 9. The proxy crate outside FUSE and `/tool`

`src/requests.rs`'s `Kind` and `src/ws.rs` gain the five FUSE answer
paths (`list`, `remove`, `rename`, `mkdir`, `stat`); `src/main.rs`
mounts files and directories by kind, adds the five FUSE and five
tool routes, holds a `Tool` in its state, and its crate doc names the
seven `/fuse/*` paths, the agent's five and the tool's five.

## 10. Amendments to report 15

Three commits in the range amend `STATUS_REPORT_15.md`, so that it
reads as follows at this commit:

- Its anchor is `47b195df2`, and the range it reports is
  `a8a982245..47b195df2`: forty-two commits, 255 files.
- Section 3.7: the cc container needs no change for FUSE mounts — by
  ruling it says nothing about credentials, and Claude Code rewrites
  its credentials file in place, which a file mount permits.
- Section 8.3: the codex container's authentication is a mounted
  `$CODEX_HOME/auth.json`, served through a FUSE mount and never read
  by the harness, then the vault's `OPENAI_API_KEY` in the
  environment; both absent refuses the run naming both; the vault's
  `OPENAI_CODEX_OAUTH` document is not a source, by ruling.
- Its closing list no longer names the codex and cc containers as
  awaiting a credentials change.

## 11. Workspace and tooling

Unchanged. `version.sh` still lists neither `diverge-agentic-loop-
codex` nor `diverge-agentic-loop-python`, both at `2.3.0`; a bump run
through the script would leave those two behind. The proxy crate's
`rmcp` feature change resolves within the existing `Cargo.lock`.

## 12. What remains

Unchanged from report 15: the spec site's prose; a live run of
anything; `version.sh` entries for the codex and python crates; image
builds of the eliza, codex and python containers, for which each
`HARNESS.md` enumerates the facts to be established.

Retired by this range: the caller-side handling of the FUSE channels
(section 3); the provider-side rendering of the FUSE mounts into the
proxy's environment (section 7.2); the server handles and the
registry's source (sections 6 and 7); the SDK's reference agent module
(section 3.4).

Added by this range: a provider implementing `ContainerDeployer`
under the restated contract, `Container`, `ContentStore`,
`ImageRegistry` with its HTTP, `VolumeManager`, `ImageChecker` and
`UnbrokeredAuthorizer`, and a caller implementing the eight
answerers — the first execution of either half; the tool container
images' MCP servers confirmed at `/mcp` on `PORT`; the FUSE directory
mounts exercised against the rename-writers named in section 2.6.
