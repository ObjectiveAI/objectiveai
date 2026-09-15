# DIVERGE PROVIDER PROTOCOL CONFORMANCE AGREEMENT

**Specification Revision 2.3.0**

This Diverge Provider Protocol Conformance Agreement (the
"Agreement") is entered into as of the Effective Date stated in the
signature block below (the "Effective Date") by and between:

**DIVERGE**, meaning [LEGAL NAME OF THE PUBLISHING ENTITY], a
[ENTITY TYPE] organized under the laws of [JURISDICTION], with its
principal place of business at [ADDRESS] ("Diverge"); and

**THE PROVIDER**, meaning [LEGAL NAME OF THE PROVIDER], a [ENTITY
TYPE] organized under the laws of [JURISDICTION], with its principal
place of business at [ADDRESS] (the "Provider").

Diverge and the Provider are each a "Party" and together the
"Parties".

## RECITALS

A. Diverge publishes the Diverge Provider Protocol Specification at
`https://protocol.diverge.network`, revision 2.3.0 (the
"Specification"), which defines the conduct of a party that serves
containers, volumes and images to clients over the protocol.

B. The Provider wishes to operate a Server under the Specification
and to hold itself out to Clients as conforming to it.

C. The Parties intend that every requirement the Specification states
of a Server be a binding contractual obligation of the Provider,
enforceable as stated in this Agreement, and that no requirement be
left to inference.

NOW, THEREFORE, in consideration of the mutual promises set forth in
this Agreement, and for other good and valuable consideration, the
receipt and sufficiency of which are hereby acknowledged, the Parties
agree as follows.

## ARTICLE 1. DEFINITIONS

Capitalized terms have the meanings stated in this Article. A term
defined in the Specification and not defined here has the meaning the
Specification gives it. Where this Agreement and the Specification
each define a term, this Agreement governs.

1.1 **"Specification"** means the Diverge Provider Protocol
Specification, revision 2.3.0, as published at
`https://protocol.diverge.network/2.3.0/` on the Effective Date,
comprising every page of that revision under the layers Overview,
WebSocket, Frames, Authorization, Container Proxy, Container Proxy
Endpoints and Endpoints, and
every Rust source file of the Reference Crate that those pages
include by reference. A copy of the Specification as of the Effective
Date is attached as Exhibit A and is incorporated into this Agreement
in full.

1.2 **"Reference Crate"** means the Rust crate `diverge-provider-sdk`
at version 2.3.0, whose source files the Specification includes. The
Reference Crate is incorporated only to the extent the Specification
includes a file of it; the Reference Crate does not otherwise bind the
Provider, and the Provider is not required to use it.

1.3 **"Protocol"** means the wire protocol the Specification defines:
the WebSocket transport, the nine-byte frame header and seven frame
types, the authorization handshake, the thirteen endpoints and their
channels, and the payload forms the Specification states for each.

1.4 **"Server"** means the party that, on one WebSocket connection,
holds the role the Specification calls the server: the party that
answers requests, finishes scopes, and opens the channels the
Specification says the server opens. The Provider is the Server on
every connection it accepts or dials under this Agreement.

1.5 **"Client"** means the party that, on one WebSocket connection,
holds the role the Specification calls the client. A Client is any
person or program that holds that role on a connection with the
Provider, whether or not it is a party to any agreement with the
Provider or with Diverge.

1.6 **"Connection"** means one WebSocket connection between the
Provider and a Client, from the completion of the WebSocket opening
handshake to its close.

1.7 **"Identity"** means the opaque value the Provider derives from a
Client's authorization credential on a Connection, as the
Specification's Authorization layer provides. Every scope on a
Connection is served under the Identity of that Connection.

1.8 **"Scope"**, **"Channel"**, **"Request"**, **"Response"**,
**"Response Finish"**, **"Channel Request"**, **"Channel Response"**
and **"Channel Response Finish"** have the meanings the
Specification's Frames layer gives them. **"Bare Finish"** means a
Response Finish that no Response precedes, or a Channel Response
Finish that no Channel Response precedes.

1.9 **"Endpoint"** means one of the thirteen scope-opening requests
the Specification's Endpoints layer defines, designated by its tag
byte: `0` `containers::agents::run`, `1` `containers::tools::run`, `2`
`containers::tools::connect`, `3` `volumes::list`, `4`
`volumes::stat`, `5` `volumes::watch`, `6` `volumes::create_capacity`,
`7` `volumes::create`, `8` `volumes::edit_capacity`, `9`
`volumes::edit`, `10` `volumes::delete`, `11` `images::check`, `12`
`version`.

1.10 **"Container Image"** or **"Image"** means an OCI image named in
a container request by one of the three forms the Specification
defines: an image the Client holds (`client`), an image the Provider
obtains from a source of its own (`server`), or a reference the
Provider pulls (`registry`).

1.11 **"Container"** means one running instance of a Container Image,
created by the Provider in performance of a `containers::agents::run`
or `containers::tools::run` request, together with the Container Proxy
running inside it, the mounts the request names, and the resource
limits the request states.

1.12 **"Container Deployment"** or **"Deployment"** means the act by
which the Provider causes a Container to exist and to run: the
instantiation of the Container Image as an isolated process
environment on computing infrastructure the Provider controls or
procures, with (a) the memory limit and the writable-disk limit the
request states applied as ceilings, (b) every Volume Mount, Identity
Mount and FUSE Mount of the request made present at its stated path
as the Specification requires, (c) the Container Proxy placed inside
the environment and started, and (d) TCP port 14979 of the environment
made reachable to the Provider's Server. A Deployment is complete when
the Container Proxy accepts a WebSocket connection at that port. A
Deployment that does not reach completion is a failed Deployment.

1.13 **"Deployment Infrastructure"** means the hardware, virtual
machines, container runtimes, orchestration systems, networks and
storage on which the Provider performs a Container Deployment, whether
owned by the Provider, leased by it, or supplied by a third party
under contract with it.

1.14 **"Container Proxy"** or **"Proxy"** means the program whose
external interface the Specification's Container Proxy layer defines
and whose scopes and channels its Container Proxy Endpoints layer
states:
the program that listens on TCP port 14979 of a Container, accepts one
WebSocket connection there from the Provider's Server — a connection
that is no connection of the Protocol, to which the Specification's
WebSocket and Frames layers apply as the Container Proxy layer
incorporates them, with the Provider's Server as the client and the
Proxy as the server, and to which the Authorization layer does not
apply — and speaks over it the scopes and channels that layer
states. The Provider satisfies the requirement to use the
Container Proxy by placing inside every Container a program that
conforms to that layer in full; the program published by Diverge under
the name `diverge-container-proxy` at version 2.3.0 is such a
program.

1.15 **"Volume"** means a directory of persistent storage the Provider
holds under a name for one Identity, as the Specification's volume
endpoints define: created by `volumes::create`, listed by
`volumes::list`, and ended only by `volumes::delete`.

1.16 **"Volume Mount"**, **"Identity Mount"** and **"FUSE Mount"**
mean, respectively, an entry of `volume_mounts`, of
`identity_file_mounts` or `identity_directory_mounts`, and of
`fuse_file_mounts` or `fuse_directory_mounts` of a container request,
as the Specification defines them.

1.17 **"Content Identity"** means the string `<size>:<hash>` by which
an Identity Mount names content, where `<size>` is the length in bytes
and `<hash>` is the SHA-256 digest, encoded as base64url without
padding, of the bytes of a file or of the manifest of a directory, the
manifest being formed as the Specification's `volumes::stat` response
page defines it.

1.18 **"Client Content"** means every byte a Client, a Container of a
Client, or a Connector transmits to the Provider under the Protocol,
including image manifests and blobs, mounted content, the bytes of
writes and reads, filesystem trees, database traffic, commands and
their items, vault keys and values, FUSE operations and their
answers, MCP exchanges, prompts, agent values, and loop chunks.

1.19 **"Relayed Exchange"** means any exchange the Specification
requires the Provider to carry between a Container and a Client, or
between a Client and a Container, without reading it: every channel
the Proxy opens on a begin Scope and every ask a FUSE Mount makes on
its Scope, with the Client's answer; the content of a Client's write;
the bytes of a read; a filetree; a database connection; and the MCP
exchanges into a tool Container.

1.20 **"Runner"** means the Client on whose `containers::tools::run`
scope a Container is running. **"Connector"** means a Client that
opens a `containers::tools::connect` scope naming that Container.

1.21 **"Obligation"** means each requirement this Agreement imposes on
the Provider, including every requirement incorporated from the
Specification under Article 2.

1.22 **"Conforming"** describes conduct of the Provider that satisfies
every Obligation applicable to it. **"Non-Conformance"** means any
failure to satisfy an Obligation.

## ARTICLE 2. INCORPORATION AND READING OF THE SPECIFICATION

2.1 **Incorporation.** Every sentence of the Specification that states
what the server, the provider, or the proxy does, does not do, sends,
does not send, holds, refuses, ignores, relays, verifies, mounts,
stops or ends is incorporated into this Agreement as an Obligation of
the Provider, as if set out in full in Article 5, and the Provider
agrees to perform each such sentence exactly as written.

2.2 **The Server is the Provider.** Wherever the Specification uses
the words "the server", "a server", "the provider" or "a provider",
those words mean the Provider. Wherever the Specification uses "the
proxy", it means the Container Proxy the Provider has placed inside a
Container, and every requirement stated of the proxy is an Obligation
of the Provider to place inside every Container a program that meets
it.

2.3 **The Client is not bound.** No sentence of the Specification
imposes an obligation on a Client under this Agreement. Where the
Specification describes what a client does, may do, or sends, that
description is a statement of the input the Provider must accept and
serve; it is not a condition of the Provider's performance and it is
not a warranty by Diverge of any Client's conduct.

2.4 **Reading of the register.** The Specification is written in the
present indicative. Each such sentence is read as a covenant of the
Provider: "The server sends exactly one response" means "The
Provider shall send exactly one response and shall not send a second".
A sentence stating what the server "may" do grants the Provider a
permission and imposes no duty. A sentence stating that a matter "is
not prescribed", "is the server's to decide", "is the server's own" or
"is not stated" reserves that matter to the Provider's discretion,
subject to every other Obligation. A sentence stating what the server
"never" does, "does not" do, or "sends no" is a prohibition. A
paragraph or clause of the Specification that begins with the word
"Remark." is descriptive: it imposes no Obligation and grants no
permission, and no Obligation is inferred from it.

2.5 **Literal values.** Every byte value, tag, port number, byte order,
width, string, JSON form and count the Specification states is exact.
The Provider shall not substitute another value, and no course of
dealing or industry practice varies a stated value.

2.6 **Order of precedence.** If this Agreement's Articles conflict
with a sentence of the Specification, the Articles govern. If a page
of the Specification conflicts with a Rust file it includes, the page
governs. If two pages of the Specification conflict, the page of the
lower-numbered layer governs as to the matter that layer owns. A
conflict exists only where two provisions cannot both be performed; a
provision that adds a requirement does not conflict with one that is
silent.

2.7 **Revision.** This Agreement binds the Provider to revision 2.3.0
of the Specification and to no other. A later revision binds the
Provider only under a written amendment to this Agreement signed by
both Parties that names the revision.

## ARTICLE 3. SCOPE OF THE PROVIDER'S SERVICE

3.1 **Conformance.** From the Effective Date and for the Term, the
Provider shall operate every Server it holds out to Clients as
conforming to the Specification in a Conforming manner, on every
Connection, for every Scope, for every Channel, without exception.

3.2 **Holding out.** The Provider shall not represent to any Client
that a Server conforms to the Specification, or to the Diverge
Provider Protocol, or to revision 2.3.0, unless that Server is
Conforming.

3.3 **Every Connection.** The Obligations apply to every Connection on
which the Provider holds the Server role, whether the Provider
accepted the Connection or dialed it, and whether or not the Client is
known to the Provider, is a customer of the Provider, or has agreed to
any terms with the Provider.

3.4 **No selective conformance.** The Provider shall not conform to
some Endpoints and not others, to some Channels and not others, or on
some Connections and not others. A Server that serves any Endpoint of
the Protocol serves every Endpoint as the Specification states.

## ARTICLE 4. DEPLOYMENT FREEDOM AND RESPONSIBILITY

4.1 **Freedom of means.** The Provider may perform Container
Deployment by any means and on any Deployment Infrastructure it
chooses: on hardware it owns or operates; on virtual machines,
container runtimes or orchestration systems it operates; on cloud
computing services or other infrastructure supplied by a third party;
through an intermediary, broker or resale arrangement in which the
Provider stands between the Client and the party that operates the
infrastructure; or by any combination of these. The Specification
prescribes the observable conduct of the Server and the Container
Proxy and prescribes no runtime, no operating system, no virtualization
technology, no network topology, and no vendor.

4.2 **Limits of the freedom.** Section 4.1 does not relieve the
Provider of any Obligation. Whatever the means, the Container Proxy
shall be reachable at TCP port 14979 of the Container to the
Provider's Server; the limits, mounts and orderings stated in Article
5 shall be observed; and every Obligation of relay, verification,
holding and ending shall be performed.

4.3 **Responsibility for third parties.** Where the Provider performs
any Obligation through a subcontractor, affiliate, cloud service or
other third party, the Provider remains solely responsible to Diverge
and to Clients for that Obligation as if it had performed it itself.
An act or omission of such a third party that would be a
Non-Conformance if committed by the Provider is a Non-Conformance of
the Provider.

4.4 **Provider's discretion.** Every matter the Specification leaves to
the Provider — among them which image references its policy allows,
which volumes it offers a Client beyond those the Client created,
whether a taken volume name is refused on creation, what a watch
observes when its volume is deleted, when a watch ends without an
error, the content of every error value, the form of a volume name it
will accept, and the source from which it obtains an image of kind
`server` — is within the Provider's discretion, and the exercise of
that discretion is not a Non-Conformance.

## ARTICLE 5. THE OBLIGATIONS

The Obligations in this Article restate the Specification's
requirements of the Server in the form of covenants. They are stated
for clarity and enforceability and do not narrow Section 2.1. Each
sub-section of this Article is a separate covenant.

### 5.1 Transport

(a) The Provider shall speak the Protocol over WebSocket as RFC 6455
defines it, version 13, adding nothing to the opening handshake,
negotiating no subprotocol, and requiring no extension.

(b) The Provider shall carry every frame of the Protocol as exactly
one WebSocket binary data frame, and shall carry no frame of the
Protocol in a text data frame or a control frame.

(c) The Provider may offer the Protocol over TLS. The use of TLS is
outside the Specification and is not an Obligation.

### 5.2 Frames

(a) The Provider shall read and write the nine-byte frame header the
Specification states: a type byte, a scope number as a `u32`
big-endian, and a channel number as a `u32` big-endian, followed by
the payload.

(b) The Provider shall treat a frame whose type byte is not one of the
seven defined values as malformed, shall disregard it, and shall not
end the Connection on account of it.

(c) The Provider shall ignore every header field the Specification
states is unread for a given frame type, and shall not prescribe a
value for it.

(d) The Provider shall acknowledge nothing at the frame layer, shall
impose no timeout at the frame layer, and shall treat a stream as
ended at its finish frame and nowhere else.

(e) The Provider shall send exactly one Response Finish on each
Scope, and only the Provider shall send it. The Provider shall finish
every Channel that a Client opens, and shall finish every Channel the
Provider opens only through the Client's Channel Response Finish, as
the Specification's rule that only the responder finishes a channel
provides.

(f) The Provider shall number the Channels it opens in its own
numbering space, independent of the Client's, and shall reuse a
Channel number of its own only after that Channel has finished.

(g) The Provider shall treat a Bare Finish it sends as its statement
that the exchange could not be served, and shall send one in every
case the Specification requires and in no case where the
Specification requires a Response.

### 5.3 Authorization

(a) On a Connection the Client dialed, the Provider shall treat the
first frame as the Client's authorization credential and shall accept
no other frame before it.

(b) The Provider shall judge an unbrokered credential by its own
means, derive the Identity from an accepted credential, and serve
every Scope on the Connection under that Identity.

(c) The Provider shall answer a rejected credential by closing the
Connection with no frame, and shall answer a credential frame received
after the handshake by ending the Connection.

(d) The Provider shall treat the Identity as opaque: it shall not
transmit it on the wire, and it shall not derive a volume name, a
container id, or any other Protocol value from it in a manner that
discloses it.

### 5.4 Endpoints in general

(a) The Provider shall read the first byte of every Request payload as
the Endpoint tag and shall serve the Endpoint that tag designates.

(b) The Provider shall answer a Request whose payload it cannot read —
a tag it does not define, or content that does not conform to the
Endpoint's form — by a Bare Finish, and by nothing else.

(c) The Provider shall read JSON payloads under the Notation the
Specification states, and postcard payloads under the postcard reading
the Specification states, and shall not require any member, field or
byte the Specification does not require.

(d) Where the Specification states that the Provider ignores bytes
that follow a request, the Provider shall ignore them and shall not
treat the request as malformed on their account.

### 5.5 `version` (tag 12)

The Provider shall answer every `version` request with exactly one
Response whose payload is the string `2.3.0` encoded as UTF-8 with no
tag and no length prefix, followed by the Response Finish. The
Provider shall not send any other string, shall not send an empty
string, and shall not send an error on this Endpoint.

### 5.6 `images::check` (tag 11)

The Provider shall answer every `images::check` request with exactly
one Response and the Response Finish: the byte `0` followed by exactly
`{"type":"available"}` when, at the time of the answer, the Provider
would supply the named image as the `server` variant of a container
request; the byte `0` followed by exactly `{"type":"unavailable"}` when
it would not; or the byte `1` followed by an error when it did not
determine the answer. An available answer reserves nothing.

### 5.7 Volumes

(a) **Listing.** The Provider shall answer every `volumes::list`
request with exactly one Response: the byte `0` followed by the
listing in the postcard form the Specification states, or the byte
`1` followed by an error. The listing shall contain every Volume the
Identity created by `volumes::create` and has not deleted by
`volumes::delete`; it may contain other Volumes at the Provider's
discretion; every Volume it contains shall be available to the
Identity to mount; no two Volumes in one listing shall share a name.
The Provider shall report for each Volume its `name`, its `bytes`, and
its `created` as the Specification defines them, and nothing more.

(b) **Stat.** The Provider shall answer every `volumes::stat` request
naming a Volume in the Identity's listing with exactly one Response
carrying the Volume's `name`, `bytes` and `created`, its `bytes_used`,
and its `dirhash` as the Specification defines each, as of the time of
the Response; and shall answer a name not in the listing with an
error.

(c) **Capacity.** The Provider shall answer every
`volumes::create_capacity` request with the largest size in bytes of a
Volume the Identity could create at the time of the Response, and
every `volumes::edit_capacity` request naming a Volume in the
Identity's listing with the number of bytes by which that Volume's
size could be increased at the time of the Response, each as a
postcard varint after the byte `0`, or an error after the byte `1`.
Neither answer reserves anything.

(d) **Creation.** The Provider shall answer every `volumes::create`
request with exactly one Response: the byte `0` only after a Volume
of the stated name and size exists for the Identity; the byte `1` when
the Provider cannot reserve the size stated; or the byte `2` followed
by an error for any other reason. From the moment the Provider sends
the byte `0`, every listing the Provider sends the Identity shall
contain the Volume until it is deleted. A Volume shall be created
empty, and the Provider shall not write to, remove from, or otherwise
alter the content of a Volume on its own account: only a Container in
which the Identity mounts the Volume changes its content. The
Provider shall refuse, by the byte `2`, a creation whose name is
already in the Identity's listing.

(e) **Editing.** The Provider shall answer every `volumes::edit`
request naming a Volume in the Identity's listing with exactly one
Response: the byte `0` only after the Volume has the size stated; the
byte `1` when the Provider cannot reserve the size stated; the byte
`2` when the content of the Volume exceeds the size stated; or the
byte `3` followed by an error for any other reason. The size is the
only property an edit changes. From the moment the Provider sends the
byte `0`, every listing and every stat the Provider sends the Identity
shall report the new size.

(f) **Deletion.** The Provider shall answer every `volumes::delete`
request naming a Volume in the Identity's listing with exactly one
Response: the byte `0` only after the Volume no longer exists; the
byte `1` when the Volume is mounted in a running Container at the
time of the request; or the byte `2` followed by an error for any
other reason. The Provider shall not delete a Volume that is mounted
in a running Container at the time of the request. From the moment
the Provider sends the byte `0`, every listing the Provider sends the
Identity shall omit the Volume, and no listing shall contain a Volume
that has been deleted. An error shall leave the Volume, its content
and the listing as they were.

(g) **Watching.** The Provider shall serve every `volumes::watch`
request naming a Volume in the Identity's listing by sending, on the
Scope, a snapshot of the Volume's filesystem tree as its first
Response, and thereafter one Response per change to that tree, in the
order the changes occurred, in the filetree form the Specification
states, until the Scope ends. The Provider shall report a node that
comes into existence as `Inserted` with the node complete; a node that
changes in place as `Modified` with the node's complete new value; a
node that ceases to exist as `Removed`, a directory with everything
beneath it and no Response for a descendant; and a node that is
relocated, by any rename or move, as `Removed` at the path it left
followed by `Inserted` at the path it arrived at with the node
complete. The Provider shall send a further snapshot whenever it has
lost track of changes, and shall send an error as its last Response
when it can no longer keep the watch. The Provider shall end the watch
by the Response Finish after the Client's stop channel request, after
the Client's Connection ends, and after an error; the Provider may end
it at another time by the Response Finish with no error. The Provider
shall send no frame on the stop channel.

(h) **Identity of the Volume namespace.** The Provider shall resolve
every Volume name against the Identity of the Connection on which it
is named and against no other. A Client shall not be able to name,
mount, stat, watch, edit or delete a Volume of another Identity.

### 5.8 Container Deployment — `containers::agents::run` (tag 0) and `containers::tools::run` (tag 1)

For every container run request the Provider accepts, the Provider
shall perform the following, in the order stated, and shall not
perform a later step before the earlier step is complete:

(a) **Read and refuse.** The Provider shall read the request. It shall
answer a request whose payload does not decode by a Bare Finish. It
shall answer by exactly one Response, the error, and the Response
Finish, a request that names a Volume not in the Identity's listing;
a mount whose path is the root of the Container; two mounts with one
path; a mount inside a FUSE directory mount; two FUSE mounts with one
`id`; a mount path with a component that is empty, `.` or `..`; or an
image source the Provider's policy does not allow. The Provider shall
hold every Volume a run request names in `volume_mounts` from the
moment it accepts the request until the run ends, and shall answer a
request that names a Volume so held by another running Container of
the same Identity, or that names one Volume twice, by exactly one
Response, the byte `1` followed by the name of that Volume as JSON,
and the Response Finish, fetching nothing and deploying nothing for
it. A Volume is mounted in at most one Container of its Identity at a
time, whatever its `persist`.

(b) **Hold every Identity Mount's content.** For each Identity Mount,
the Provider shall, before Deployment, either hold content it has
verified against the Content Identity, or open a `fetch-file` or
`fetch-directory` channel on the Scope, receive what the Client sends,
and verify the received content's size and hash against the Content
Identity before treating it as held. The Provider shall treat a
channel the Client finishes with no frame, and content that does not
match its Content Identity, as the run's error. The Provider may hold
content received in an earlier run of any Identity.

(c) **Serve the Client's image.** For an image of kind `client`, the
Provider shall serve an OCI registry from which its runtime pulls the
image by repository name and manifest digest; shall ask the Client, on
an `oci-manifest` or `oci-blob` channel, for each manifest or blob
the registry does not hold, at most once per digest per run; shall
compute the digest of every manifest and blob received and store and
serve only bytes whose digest equals the digest asked for; shall serve
every byte range from its store and never by a further ask; and shall
treat a digest the Client does not hold, and bytes that do not match,
as the run's error. For an image of kind `server`, the Provider shall
obtain the image from a source of its own. For an image of kind
`registry`, the Provider shall pull the stated reference.

(d) **Deploy.** The Provider shall perform Container Deployment as
Section 1.12 defines it, with `memory` and `disk` of the request as
ceilings; every Volume Mount resolved by `host_name` against the
Identity, descended by `host_relative_path`, and made present at
`container_path`, the Container's changes to it being in the Volume
when the Container ends if `persist` is `true` and the Volume being as
it was before the run when the Container ends if `persist` is
`false`; every Identity Mount made present at its
`container_path`, with content that matches its Content Identity in
size and hash at the start of the Container's life and that is
writable from inside the Container; the Container Proxy placed inside
and started; and
TCP port 14979 reachable to the Provider's Server. The Provider shall
set no environment variable in the Container from the request, shall
expose no port of the Container other than port 14979, and shall not
start the Container before every Identity Mount's content is held. A
failed Deployment is the run's error.

(e) **Connect to the Proxy.** The Provider shall open exactly one
WebSocket connection to TCP port 14979 of the Container and, on it
before any other Scope, the begin Scope of the Container's family —
for an agent container, carrying the request's `agent` value verbatim
— and shall await its answer. A Proxy that does not accept the
connection, or that answers the begin Scope with an error, is a
Container that did not come up: the Provider shall stop the Container
and treat the failure as the run's error. The Provider shall open no
second connection to a Container.

(f) **Make every FUSE Mount.** For each entry of `fuse_file_mounts`,
and after the last of them for each entry of `fuse_directory_mounts`,
in the order of the request, the Provider shall open one mount Scope
on the Proxy's connection, naming the path and the kind, and shall
wait for its answer before opening the next. The Provider shall treat
a mount the Proxy does not make as the run's error and shall stop the
Container. The Provider shall open no tree Scope and send no Response
on the Scope before the last FUSE Mount is complete. Inside
a FUSE directory mount, every entry beneath the mount point shall be
creatable, writable, renamable and removable from inside the
Container, each change relayed to the Client as the Specification
provides; the mount point itself is not removable or renamable. A
FUSE file mount is overwritable in place and is not removable,
renamable or replaceable. Whether a change is allowed is the Client's
answer to the ask that carries it; the Provider shall enforce no
restriction of its own on a FUSE Mount.

(g) **Hold what the Client opened.** The Provider shall serve a
Channel the Client opened before the id was sent only after the id is
sent, in the order the Channels were opened, and shall neither refuse
nor read such a Channel before the last FUSE Mount is complete.

(h) **Carry the agent** (tag 0 only). For an agent container, the
Provider shall carry the request's `agent` value verbatim in the begin
Scope's request, and in nothing else. A begin the Proxy answers with
an error is the run's error, and the Provider shall stop the
Container.

(i) **Mint and send the id.** The Provider shall choose an id that is
unique among the Containers it is running and not derivable from the
request, from the Identity, or from any other id, and shall send it as
exactly one Response; a run refused for a held Volume has the byte `1`
and the name as its only Response, and a run that fails has the error
as its only Response. From that moment the Container is running for
the Client and shall be findable by a `containers::tools::connect`
request naming the id. After the id the Provider shall send, for a
tool container, no further Response on the Scope, and, for an agent
container, every chunk the Proxy sends on the begin Scope's main
stream as one Response, the byte `3` followed by the chunk verbatim,
in the order the Proxy sent them, and no other Response.

(j) **Serve the Scope.** For as long as the Scope lives, and
concurrently, the Provider shall relay every Channel the Proxy opens
on the begin Scope, and every ask a FUSE Mount makes on its Scope, to
the Client as a Channel the Provider opens, in the form the
Specification states for that ask, and shall serve every Channel the
Client opens as the Specification states for that Channel.

(k) **End the run.** When the Provider receives the Client's stop
channel request, when the Proxy's connection ends, or when the
Client's Connection ends, the Provider shall end every
connect Scope on the Container, stop the Container, release the
registry repository it served for the run, end every Channel task, and
send the Response Finish with no error. After the id, the Provider
shall send no error on the main stream of the Scope.

### 5.9 The Conduit

(a) The Provider shall relay the bytes of every Relayed Exchange
verbatim. It shall not read them, shall not alter them, shall not
reorder frames within one Channel, and shall not send an ask a second
time.

(b) The Provider shall relay each Channel the Proxy opens on the begin
Scope, and each ask a FUSE Mount makes on its Scope, as the Channel
the Specification assigns to it, carrying the ask's bytes as the
Specification states — the mount's id added in front of a FUSE Mount's
ask — and shall relay the Client's answer onto the Proxy's Channel as
Channel Responses, one per Channel Response of the Client's, finishing
the Proxy's Channel when the Client's Channel finishes.

(c) The Provider shall relay a Channel the Client finishes with no
frame before the finish to the Proxy as a Channel Response Finish that
no Channel Response precedes.

(d) For a database connection the Container opens, which the Proxy
announces on the begin Scope under an id of the Proxy's, the Provider
shall mint a connection id of its own, unique among the connections
open on the Scope, open its own `postgres` Channel to the Client
carrying that id, relay every Channel Response of that Channel onto
the Proxy's Channel verbatim, and finish the Proxy's Channel when the
Client finishes; and, when the Client opens its half quoting the
Provider's id, open the Provider's own half on the begin Scope quoting
the Proxy's id and relay everything the Proxy answers with onto the
Client's half, in order, finishing the Client's half when the Proxy
finishes.

(e) For a write the Client opens, the Provider shall open a
`write-bytes` Channel quoting the Client's write id, open a write
Scope on the Proxy's connection for the stated path, answer the
Channel the Proxy opens on that Scope with each piece of content as
it arrives, finish that Channel when the Client's Channel finishes,
and answer the Client's write Channel with exactly one Channel
Response, written or an error, after the Proxy has answered the
Scope. Content that ends in an error or without a finish is a write
that did not happen, and the Provider shall answer it as such.

(f) For a filetree the Client opens, the Provider shall open a tree
Scope on the Proxy's connection naming, in the request, the path of
every Volume Mount, Identity Mount and FUSE Mount of the Container,
shall relay every frame the Proxy sends, and shall stop the tree
Scope when the Client's Scope ends.

### 5.10 `containers::tools::connect` (tag 2)

For every connect request, the Provider shall, in order: (a) read the
request and answer a payload that does not decode by a Bare Finish;
(b) look the `id` up among the Containers it is running and answer an
id that names none by exactly one Response, the error
`{"kind":"missing"}`, and the Response Finish; (c) open an `authorize`
Channel on the run Scope of the Container carrying the peer IP
address of the Connector's Connection as the Provider observed it and
the request's `authorization` string verbatim, read the Runner's
answer, and answer the byte `0`, any byte other than `1`, a Bare
Finish, or a Runner that is gone by exactly one Response, the error
`{"kind":"denied"}`, and the Response Finish; (d) on the byte `1`,
send nothing on the main stream, and serve every Channel the Connector
opens on the run's own connection to the Container's Proxy — its
tree, read and write Scopes, and its MCP exchanges as Channels on the
run's begin Scope — as for a run, except that a
Connector's `postgres` Channel shall be answered by a Bare Finish,
that the only Channel the Provider opens on a Connector is
`write-bytes` for the Connector's own writes, and that a filetree the
Connector opens leaves out every mount of the run; (e) end the Scope
by the Response Finish with no error when the Connector disconnects,
when the run ends, or when the Connector's Connection ends, stopping
nothing and releasing nothing. The Provider shall not send anything on
the connect Scope, and shall serve no Channel of it, before the Runner
has answered.

### 5.11 Prohibitions

The Provider shall never: (a) refuse, alter or withhold a FUSE ask on
its own account, whether a change to a FUSE Mount is allowed being the
Client's answer to that ask; (b) write to a Volume or a mount on its
own account; (c) send a second id on a run Scope; (d) send an error on
the main stream of a run Scope after the id; (e) retry any ask; (f)
read, inspect, parse, log the content of, or act upon the content of
a Relayed Exchange, save to the extent necessary to relay it; (g)
impose a timeout on any fetch, Deployment, Channel, Scope or
Connection; (h) send a Response where the Specification states a Bare
Finish, or a Bare Finish where the Specification states a Response;
(i) mint a container id or a connection id that is derivable from
anything a Client chose; (j) serve a Scope under an Identity other
than that of the Connection on which the Scope was opened; or (k)
mount an Identity Mount read-only, or otherwise cause a write to it
from inside the Container to fail. Whether a write to an Identity Mount outlives the
Container is not prescribed.

### 5.12 The Container Proxy

The Provider shall place inside every Container a Container Proxy
that conforms in full to the Specification's Container Proxy and
Container Proxy Endpoints layers:
that listens on TCP port 14979 and accepts one WebSocket connection
there; that speaks the Protocol's own frames on it and sends no auth
frame; that answers exactly one begin Scope per connection, holding
the agent it carries for the Container's life; that makes each mount
Scope's mount before answering it and holds every mount for its life,
asking for what the mount needs on channels of that Scope; that
leaves out of every filetree the paths the tree Scope's request names
and `/proc`, `/sys` and `/dev`; that ignores text data frames; and
that imposes no timeout. Where the Provider uses a program other than
the one Diverge publishes, the Provider warrants that program's
conformance.

## ARTICLE 6. CLIENT CONTENT

6.1 **Use limited to performance.** The Provider shall use Client
Content only to perform its Obligations. The Provider shall not
disclose Client Content to any third party except a subcontractor
bound by written obligations of confidentiality no less protective
than this Article and engaged in performing an Obligation.

6.2 **Retention.** The Provider may retain, after a run ends, only
(a) image manifests and blobs it has verified against their digests,
and (b) Identity Mount content it has verified against its Content
Identity, each for use in a later run as the Specification permits;
and (c) Volumes, for as long as the Specification requires. The
Provider shall not retain any other Client Content after the Scope
that carried it ends, and shall not retain the content of a Relayed
Exchange at any time except transiently in the course of relaying it.

6.3 **No inspection.** The Provider's Obligation under Section 5.9(a)
not to read Relayed Exchanges is a term of this Article as well as of
Article 5.

6.4 **Compelled disclosure.** If the Provider is required by law,
regulation, subpoena or court order to disclose Client Content, it
shall, to the extent legally permitted, give Diverge and the affected
Client prompt written notice before disclosure and shall disclose
only what is required.

## ARTICLE 7. WARRANTIES

7.1 **Provider's warranties.** The Provider represents and warrants
that: (a) it has full power and authority to enter into and perform
this Agreement; (b) every Server it holds out as conforming is and
will remain Conforming for the Term; (c) every Container Proxy it
places inside a Container conforms to Section 5.12; (d) its
performance of this Agreement does not and will not violate any
agreement to which it is a party or any applicable law; and (e) the
Deployment Infrastructure it uses is adequate to perform every
Obligation.

7.2 **Diverge's warranties.** Diverge represents and warrants that it
has full power and authority to enter into this Agreement and that it
is the publisher of the Specification.

7.3 **Disclaimer.** EXCEPT AS EXPRESSLY STATED IN THIS ARTICLE, THE
SPECIFICATION AND THE REFERENCE CRATE ARE PROVIDED "AS IS", AND
DIVERGE DISCLAIMS ALL OTHER WARRANTIES, EXPRESS OR IMPLIED, INCLUDING
THE IMPLIED WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR
PURPOSE AND NON-INFRINGEMENT.

## ARTICLE 8. CONFORMANCE VERIFICATION

8.1 **Records.** The Provider shall keep, for the Term and for one
year thereafter, records sufficient to demonstrate its Conforming
operation, including the version of the Container Proxy it deploys and
the Deployment Infrastructure it uses.

8.2 **Testing.** Diverge may, at any time during the Term and no more
than four times in any twelve-month period, exercise a Server the
Provider holds out as conforming, as a Client, for the purpose of
verifying conformance. The Provider shall serve such exercise as it
serves any Client.

8.3 **Notice of Non-Conformance.** The Provider shall notify Diverge in
writing within five business days after it becomes aware of any
Non-Conformance of a Server it holds out as conforming, stating the
Obligation affected, the period of the Non-Conformance, and the
remediation.

## ARTICLE 9. REMEDIES

9.1 **Material breach.** Any Non-Conformance is a material breach of
this Agreement.

9.2 **Cure.** On written notice from Diverge specifying a
Non-Conformance, the Provider shall cure it within thirty days, or
within such shorter period as the notice states where the
Non-Conformance exposes Client Content or endangers a Client's
Volumes or Containers, and shall during the cure period cease to hold
out the affected Server as conforming.

9.3 **Equitable relief.** The Provider acknowledges that a
Non-Conformance would cause Diverge and Clients irreparable harm for
which damages are an inadequate remedy, and agrees that Diverge is
entitled to specific performance and to temporary, preliminary and
permanent injunctive relief to compel conformance or to restrain a
Non-Conformance, without the posting of a bond and in addition to
every other remedy available at law or in equity.

9.4 **Third-party beneficiaries.** Each Client that holds the Client
role on a Connection with the Provider is an intended third-party
beneficiary of Articles 3, 4, 5 and 6 and of Section 9.3, and may
enforce them against the Provider as to the Connections, Scopes,
Volumes and Containers of that Client. No other person is a
third-party beneficiary of this Agreement.

9.5 **Limitation.** EXCEPT FOR A BREACH OF ARTICLE 6, A BREACH OF
SECTION 3.2, OR A PARTY'S GROSS NEGLIGENCE, WILLFUL MISCONDUCT OR
FRAUD, NEITHER PARTY SHALL BE LIABLE TO THE OTHER FOR ANY INDIRECT,
INCIDENTAL, CONSEQUENTIAL, SPECIAL OR PUNITIVE DAMAGES ARISING OUT OF
THIS AGREEMENT, HOWEVER CAUSED AND UNDER ANY THEORY OF LIABILITY.
Nothing in this Section limits the relief available under Section 9.3
or the liability of the Provider to a Client under Section 9.4.

## ARTICLE 10. TERM AND TERMINATION

10.1 **Term.** This Agreement begins on the Effective Date and
continues until terminated under this Article (the "Term").

10.2 **Termination for convenience.** Either Party may terminate this
Agreement on ninety days' written notice to the other.

10.3 **Termination for breach.** Diverge may terminate this Agreement
on written notice if the Provider fails to cure a Non-Conformance
within the period Section 9.2 provides.

10.4 **Effect.** On termination the Provider shall cease to hold out
any Server as conforming to the Specification; shall, for every
Container running on a Scope open at termination, continue to perform
Article 5 until that Scope ends; and shall retain every Volume of
every Identity until the Identity deletes it or until the Provider has
given the affected Client not less than ninety days' written notice of
the Volume's removal.

10.5 **Survival.** Articles 1, 2, 6, 7.3, 9 and 11, Sections 8.1 and
10.4, and every Obligation that by its nature continues, survive
termination.

## ARTICLE 11. GENERAL

11.1 **Governing law.** This Agreement is governed by the federal laws
of the United States of America and, to the extent the law of a state
of the United States governs a matter that federal law does not, by
the laws of the State of Delaware, without regard to its conflict-of-
laws principles. The United Nations Convention on Contracts for the
International Sale of Goods does not apply.

11.2 **Venue.** Each Party irrevocably submits to the exclusive
jurisdiction of the United States District Court for the District of
Delaware, and, only if that court lacks subject-matter jurisdiction,
of the state courts of the State of Delaware, for any action arising
out of this Agreement, and waives any objection to venue in those
courts.

11.3 **Electronic signatures.** This Agreement may be signed
electronically. An electronic signature, and a record of this
Agreement kept in electronic form, has the same legal effect as a
manual signature and a paper record under the Electronic Signatures in
Global and National Commerce Act, 15 U.S.C. § 7001 et seq.

11.4 **Notices.** Every notice under this Agreement shall be in
writing and delivered to the address stated in the signature block, or
to such other address as a Party designates by notice, by courier, by
certified mail, or by electronic mail with confirmation of receipt,
and is effective on receipt.

11.5 **Assignment.** The Provider shall not assign this Agreement, or
delegate any Obligation, without Diverge's prior written consent,
except as Article 4 permits performance through third parties. Any
attempted assignment in violation of this Section is void.

11.6 **Entire agreement.** This Agreement, with Exhibit A, is the
entire agreement of the Parties on its subject and supersedes every
prior or contemporaneous agreement, representation or understanding on
that subject.

11.7 **Amendment; waiver.** This Agreement may be amended only by a
written instrument signed by both Parties. No waiver is effective
unless in writing, and no waiver of one breach is a waiver of another.

11.8 **Severability.** If a provision of this Agreement is held
unenforceable, it shall be enforced to the maximum extent permitted,
and the remaining provisions remain in full force. Without limiting
the foregoing, if any Obligation is held unenforceable, every other
Obligation remains binding.

11.9 **Independent parties.** The Parties are independent contractors.
Nothing in this Agreement creates a partnership, joint venture, agency
or employment relationship.

11.10 **Counterparts.** This Agreement may be executed in
counterparts, each of which is an original and all of which together
are one instrument.

11.11 **Headings; construction.** Headings are for convenience only.
"Including" means "including without limitation". No rule of
construction against the drafter applies.

## SIGNATURES

IN WITNESS WHEREOF, the Parties have executed this Agreement as of the
Effective Date.

**DIVERGE**

By: ______________________________
Name: ____________________________
Title: ___________________________
Date: ____________________________
Address for notices: _____________

**THE PROVIDER**

By: ______________________________
Name: ____________________________
Title: ___________________________
Date: ____________________________
Address for notices: _____________

Effective Date: ___________________

## EXHIBIT A — THE SPECIFICATION

The Diverge Provider Protocol Specification, revision 2.3.0, as
published at `https://protocol.diverge.network/2.3.0/` on the
Effective Date, is attached to this Agreement as the Markdown
rendering the site publishes beside each page of the revision, the
pages being those the site's sitemap at
`https://protocol.diverge.network/sitemap-index.xml` enumerates under
`/2.3.0/`, with every included Rust source file expanded in place, and
initialed by both Parties on each page.
