# Report 13: the OCI pull as two typed exchanges

## The problem with `oci` as it stands

`shared::oci` is a tunnel. When a run names an `Image::Client`, the
provider stands up a registry endpoint, points its container runtime
at it, and every HTTP request the runtime makes that the provider
cannot answer is copied, bytes for bytes, into a channel toward the
caller — request line, headers and all — and the caller's answer is
copied back the same way. The crate reads none of it, on the argument
that the OCI Distribution specification's semantics ARE HTTP and a
typed form would be re-specifying somebody else's spec.

That argument was right about pushes and wrong about pulls. A pull
uses two endpoints, both of them a fetch by identity, and everything
HTTP adds on top — `Accept` negotiation, `Range`, `HEAD`, `Docker-
Content-Digest`, redirects, token auth — is the concern of the party
SERVING the registry, which under this design is the provider, not the
caller. Tunneling it makes the caller implement a registry it has no
other reason to have, makes the wire carry headers nobody on the far
end wants, and leaves the provider unable to cache, verify or resume
anything because it never learns what a request was for.

The same shape that replaced the tunneled MCP exchange fixes this: say
what is being asked instead of carrying a request that says it.

## The shape

The provider runs a real registry — the read side of the OCI
Distribution API, served over plain HTTP on the provider's loopback —
and the container runtime pulls from it as from any registry. Behind
it, the provider holds a content-addressed store. Whatever the store
does not hold, the provider asks the caller for over the scope, by
digest, with a typed channel request, exactly as it asks for a mounted
file it does not hold. The caller answers bytes until the finish, or
finishes empty when it does not hold the digest.

The caller therefore needs no HTTP at all. It needs a store of
manifests and blobs indexed by digest, which is what an image IS once
it has been saved anywhere — a `dir:` from skopeo, an OCI layout, a
`podman save` archive unpacked — and the same kind of store it already
keeps for hash mounts.

## The pull, endpoint by endpoint

What a runtime does to pull `Image::Client { name, digest }`, and
what the provider's registry needs from the caller to answer each
step. The runtime is podman, containerd or docker; they differ in
order and in whether they `HEAD` first, and none of it changes the
set.

| the runtime sends | the registry answers | needs from the caller |
|---|---|---|
| `GET /v2/` | `200`, the API version | nothing |
| `HEAD /v2/<name>/manifests/<digest>` | `200`, `Content-Type`, `Content-Length`, `Docker-Content-Digest` | the manifest, if not held |
| `GET /v2/<name>/manifests/<digest>` | `200`, the manifest, its media type as `Content-Type` | the manifest, if not held |
| the manifest is an index | the runtime picks its platform's manifest by digest and asks again | that manifest, if not held |
| `HEAD /v2/<name>/blobs/<digest>` | `200`, `Content-Length` | nothing — the size is in the manifest's descriptor |
| `GET /v2/<name>/blobs/<digest>` | `200`, the bytes | the blob, if not held |
| `GET …/blobs/<digest>` with `Range` | `206`, the range | nothing — served from the store, which holds the whole blob |
| token auth, redirects, `tags/list`, referrers, any push endpoint | never happens; a loopback registry needs no auth, and a pull never lists or pushes | nothing |

Two things come from the caller, and only two: a manifest by digest,
and a blob by digest. Everything else the registry answers on its own
— the ping from a constant, `HEAD`s from the store or from the
manifest's descriptors, ranges from the store, `Docker-Content-Digest`
from the digest the runtime asked by.

Tags never cross the wire. An `Image::Client` is pinned by digest,
and every reference inside a manifest is a digest, so there is no
step at which a name has to be resolved. The `<name>` segment exists
because the runtime's URL has one and routes the request; the store
is keyed by digest alone, because a manifest under one name is the
same manifest under another.

## The two exchanges

Both are the provider asking on the run scope, the shape
`fetch_file` and `fetch_directory` already have, and they sit beside
them in `shared::containers`:

- **`fetch_manifest`.** The ask names a digest. The answer is one
  frame carrying the manifest's media type and its bytes — the media
  type because the runtime needs it back as `Content-Type`, and it is
  not recoverable from the bytes (`application/vnd.oci.image.index.v1+json`
  and `…manifest.v1+json` are both JSON objects with a `mediaType`
  field that is optional). One frame, because a manifest is small.
  Then the finish. An empty finish is the caller not holding it.

  ```text
  ask:     {"digest": "sha256:…"}                         JSON
  answer:  [u16 BE: media type length][media type][manifest bytes]
  ```

- **`fetch_blob`.** The ask names a digest. The answer is the blob's
  bytes, one frame per piece, each at most `CHUNK_SIZE`, appended in
  order, then the finish — `fetch_file`'s frame exactly. An empty
  finish is the caller not holding it.

  ```text
  ask:     {"digest": "sha256:…"}                         JSON
  answer:  [bytes…] … [bytes…]                            then the finish
  ```

No offset on the blob ask. The provider fetches a blob whole, once,
and serves every range the runtime asks for out of its store; a
partial blob the provider held would be a blob it cannot verify, and
resuming across the caller's store would need the caller to answer
ranges, which is HTTP creeping back in.

No error vocabulary on either, as on the other fetches: nothing an
error could say would change what the provider does next, which is
not run the container. The `Error` the caller sees is the run scope's
own, on channel `0`.

## The provider's registry

What the server half provides, behind the `server` feature.

- **A store keyed by digest.** Manifests and blobs, verified on
  arrival: the provider hashes what the caller sent and refuses to
  store a mismatch, so a caller serving the wrong bytes under the
  right digest fails at the provider rather than at the runtime — and
  the store is safe to share across runs and across callers, because
  a digest is the content. Sizes are known before a blob is fetched,
  from the descriptor in the manifest that named it, which is how
  `HEAD` and `Content-Length` are answered without fetching.
- **A fetch by digest that goes to the store first.** A miss opens
  the exchange on the scope the pull belongs to, streams the answer
  into the store, and then serves it. Two runtimes pulling the same
  digest at once wait on one fetch.
- **The read side of the Distribution API**, over HTTP on the
  loopback: the version ping, `GET`/`HEAD` on manifests and blobs,
  `Range` on blobs. The `<name>` segment carries the scope the pull
  belongs to, as it does today, so one listener serves every run.
  Nothing else — a request for anything not in the table above is
  `404`.
- **Multi-arch.** An index is a manifest like any other; the runtime
  reads it and asks for its platform's manifest by digest, and the
  registry fetches that one too. The provider never chooses a
  platform.

The one thing to decide is where the HTTP face lives. The crate's
`axum` is deliberately `ws`-only — "so nothing here can speak HTTP at
all" — and the registry is the first thing the server half has to
speak HTTP for. Either the `server` feature grows `http1` and the
registry is this crate's, complete, or the crate provides the store
and the fetch-by-digest and a provider mounts the handful of routes
itself. The first is one place, identical for every provider, and
the argument for keeping HTTP out was about the WIRE, which stays
free of it; I would take it.

## What this replaces

- `shared::oci` — the request and response byte frames. Deleted.
- `server::client_registry` and `server::oci_stream` — the tunnel's
  server half. Deleted. `ContainerDeployer::client` no longer takes a
  `ClientRegistry`; it takes the registry's address for this scope,
  since the runtime needs a URL and nothing else.
- `Frame::Oci`, tag `0` on the run scopes' server channel requests —
  replaced by `FetchManifest` and `FetchBlob`. They join the other
  two fetches, so the provider's five own asks become six, and the
  container's asks shift up by one tag.
- The `oci` module in each run's `client::channel_response` —
  replaced by `fetch_manifest` and `fetch_blob` aliases.
- `Image::Client`'s doc, which says "the caller runs a registry and
  the provider relays to it". It becomes: the caller holds the
  manifest and the blobs, by digest, and answers for them.

The caller-facing `Image` shape does not change: `Client { name,
digest }` is still the pinned pair, and the same pair `images::check`
asks about. What changes is what holding a client image obliges a
caller to do — answer two fetches — and it is less than before.

## What the caller does

Keep a store. An image the caller built locally is already on its
disk as manifests and blobs — in podman's storage, or in an OCI
layout it exported — and the caller answers each fetch by looking up
the digest there. For a runtime-native store the lookup is a
`podman image inspect` and a read of the layer files; for an OCI
layout it is a file under `blobs/sha256/`. Neither involves running
a registry, and the caller-side executor, when it is written, takes
one trait for both fetches: `fetch_manifest(digest)` and
`fetch_blob(digest)`, each answering a stream of bytes or none — the
shape `fetch_file` already has for hash mounts.

## Summary

| exchange | who asks | by | answer |
|---|---|---|---|
| `fetch_manifest` | provider | digest | media type + bytes, one frame, or empty |
| `fetch_blob` | provider | digest | bytes, chunked at `CHUNK_SIZE`, or empty |

Everything HTTP the runtime needs — `HEAD`, `Range`, `Content-Type`,
`Content-Length`, `Docker-Content-Digest`, the version ping — is the
provider's registry answering from what those two exchanges put in
its store, and none of it crosses the wire.
