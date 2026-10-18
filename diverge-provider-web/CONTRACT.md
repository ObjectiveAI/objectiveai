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
`https://provider.diverge.network`, revision 2.3.0 (the
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
`https://provider.diverge.network/2.3.0/` on the Effective Date,
comprising every page of that revision under the layers Overview,
WebSocket, Frames, Authorization, Container Proxy, Container Proxy
Endpoints and Endpoints, and
every Rust source file of the Reference Crate that those pages
include by reference. A copy of the Specification as of the Effective
Date is attached as Exhibit A and is incorporated into this Agreement
in full.

1.2 **"Reference Crate"** means the Rust crate `diverge-sdk`
at version 2.3.0, whose source files the Specification includes. The
Reference Crate is incorporated only to the extent the Specification
includes a file of it; the Reference Crate does not otherwise bind the
Provider, and the Provider is not required to use it.

1.3 **"Protocol"** means the wire protocol the Specification defines:
the WebSocket transport, the nine-byte frame header and seven frame
types, the authorization handshake, the twenty-four endpoints and their
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

1.9 **"Endpoint"** means one of the twenty-four scope-opening requests
the Specification's Endpoints layer defines, designated by its tag
byte: `0` `containers::agents::run`, `1` `containers::tools::run`, `2`
`containers::fuse_serve`, `3` `volumes::list`, `4` `volumes::stat`, `5`
`volumes::read`, `6` `volumes::write`, `7` `volumes::filetree`, `8`
`volumes::fuse_serve`, `9` `volumes::create_capacity`, `10`
`volumes::create`, `11` `volumes::edit_capacity`, `12` `volumes::edit`,
`13` `volumes::delete`, `14` `images::check`, `15` `images::list`,
`16` `images::stat`, `17` `images::build`, `18` `images::delete`,
`19` `images::oci`, `20` `info`, `21` `capacity`, `22`
`daemons::accept`, `23` `daemons::connect`.

1.10 **"Container Image"** or **"Image"** means an OCI image named in
a container request by a manifest digest and by zero or more
references, each a registry host and a repository path, as the
Specification defines them. The digest identifies the image and a
reference locates it; where the Provider obtains the image is the
Provider's own.

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
request states applied as ceilings, (b) every Volume Mount and FUSE
Mount of the request made present at its stated path
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
`volumes::list`, and ended only by `volumes::delete`. A Volume has a
mode, `persistent`, `ephemeral` or `read_only`, stated at its
creation and changed only by `volumes::edit`, which governs what
becomes of a change a Container or a FUSE serve makes to it — kept,
discarded afterwards, or refused — and who may hold it at once, at
most: a `persistent` Volume any number of running Containers at once
or one FUSE serve at a time, never both; an `ephemeral` or a
`read_only` Volume any number of each, beside one another.
Whether the Provider keeps Volumes of a mode at all, and how much of
what the mode allows the Provider allows, is the Capability the
Provider holds out for the mode.

1.16 **"Client Image"** means an image the Provider holds under a name
for one Identity, built by the Provider from a Containerfile that
Identity's `images::build` request carried, as the Specification's
image endpoints define: listed by `images::list`, examined by
`images::stat`, and ended only by `images::delete`. A Client Image is
a Container Image the Provider holds for the Identity that built it
and for no other Identity.

1.17 **"Volume Mount"** and **"FUSE Mount"** mean, respectively, an
entry of `volume_mounts`, and of `fuse_file_mounts` or
`fuse_directory_mounts` of a container request or a mount the Client
asks for on a run Scope's `fuse_mount` Channel, as the Specification
defines them.

1.18 **"Client Content"** means every byte a Client, a Container of a
Client, or a Daemon Connector transmits to the Provider under the
Protocol,
including image manifests and blobs, the bytes of writes, reads and
transfers, filesystem trees, database traffic, commands and
their items, FUSE asks and their answers on a
mount and on a `volumes::fuse_serve`, MCP exchanges, prompts, arguments,
the names, modes and values of secrets, dependency declarations, and
loop chunks.

1.19 **"Relayed Exchange"** means any exchange the Specification
requires the Provider to carry between a Container and a Client, or
between a Client and a Container, without reading it: every channel
the Proxy opens on a begin Scope and every ask a FUSE Mount makes on
its Scope, with the Client's answer; the content of a Client's write;
the bytes of a read; the bytes of a transfer, from one Container into
another; a filetree; a database connection; the MCP exchanges
into a tool Container; and a daemon connection between a Daemon
Connector and an Acceptor.

1.20 **"Runner"** means the Client on whose `containers::agents::run`
or `containers::tools::run` scope a Container is running.
**"Acceptor"** means a Client on whose `daemons::accept` scope the
Provider announces daemon connections, under the Identity of that
scope's Connection. **"Daemon Connector"** means a Client that opens
a `daemons::connect` scope naming an Acceptor's Identity.

1.21 **"Obligation"** means each requirement this Agreement imposes on
the Provider, including every requirement incorporated from the
Specification under Article 2.

1.22 **"Conforming"** describes conduct of the Provider that satisfies
every Obligation applicable to it. **"Non-Conformance"** means any
failure to satisfy an Obligation.

1.23 **"Capability"** means one of the parts of the Specification's
Endpoints layer that a Server holds out or does not, as the
Specification's Capabilities layer defines them — the client image
store, the registries pulled from, FUSE Mounts, the FUSE serve of a
Container, and each Volume mode with what may hold a Volume of it —
stated in the `capabilities` object of every `info` Response the
Server sends, which object the Specification calls the Server's
capabilities.

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
the Provider — among them which registries its policy allows,
which volumes it offers a Client beyond those the Client created,
whether a taken volume name is refused on creation, the content of
every error value, the form of a volume name it
will accept, and the source from which it obtains an image — is
within the Provider's discretion, and the exercise of
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

### 5.5 `info` (tag 20) and the Capabilities

(a) **The answer.** The Provider shall answer every `info` request
with exactly one Response whose payload is one JSON object with
exactly two members — `version`, the string `2.3.0`, and
`capabilities`, the object the Specification's Capabilities layer
defines — with no tag and no length prefix, followed by the Response
Finish. The Provider shall not send any other `version` string, shall
not send an empty string, shall not send a member this revision does
not define, and shall not send an error or a refusal on this
Endpoint.

(b) **One object.** The Provider shall send the same `capabilities`
object in every `info` Response it sends, on every Connection, for
as long as the Server runs, and shall serve every request on every
Connection as that object states. The object shall not depend on the
Identity, on the request, or on the time.

(c) **What is held out is served.** For every Capability the
`capabilities` object holds out, the Provider shall serve the
Endpoints the Capability governs in full as the Specification states
them, and shall not send any Response the Specification defines for
that Capability's absence.

(d) **What is not held out is refused as defined.** For every
Capability the `capabilities` object does not hold out, the Provider
shall answer every request past it with exactly the Response the
Specification's Capabilities layer names for that Endpoint, as the
first and only Response on the Scope, followed by the Response Finish;
shall serve no Channel on the Scope; and shall look up nothing, hold
nothing, obtain nothing, deploy nothing and change nothing for the
request. In particular the Provider shall answer: every
`images::list`, `images::stat`, `images::build`, `images::delete` and
`images::oci` request, where `images_store` is not held out, by the
byte `4`, `1`, `4`, `2` and `0` respectively, alone; every `volumes::create` request
stating a mode the Provider holds out no mode object for by the byte
`2` alone, and every `volumes::edit` request naming a Volume in the
Identity's listing and stating such a mode by the byte `3` alone; and,
where the Provider holds out none of the three mode objects, every
request of the eleven Volume Endpoints by the byte the Specification's
Capabilities layer tabulates for it, alone — `volumes::list`,
`volumes::stat`, `volumes::read`, `volumes::write`,
`volumes::filetree` and `volumes::delete` `4`, `volumes::fuse_serve`
`5`, `volumes::create_capacity` and `volumes::edit_capacity` `1`,
`volumes::create` `3`, `volumes::edit` `7`.

(e) **The mode objects.** A mode object with at least one member
`true` is a Volume mode the Provider holds out; a mode object with
every member `false`, or absent, which is the same statement, is a
mode it does not hold out. For every Volume mode the Provider holds
out, the Provider shall send a mode object that holds out no more
than the mode allows — the `volumes_persistent` object with
`fuse_serve_many` and `mount_and_fuse_serve` `false` —
that sends `mount` `true` wherever it sends `mount_many` `true`,
`fuse_serve` `true` wherever it sends `fuse_serve_many` `true`, and
both wherever it sends `mount_and_fuse_serve` `true`; shall list to
no Identity a Volume of a mode it holds out no mode object for; and
shall decide every hold of a Volume by the mode object for the
Volume's mode, in the order the Specification's Capabilities layer
states for a run's Volume Mount, for a FUSE serve, for an operation
and for a build's context, the first rule that applies deciding, the
decision and the hold being one act.

### 5.6 `capacity` (tag 21)

(a) **The answer.** The Provider shall answer every `capacity`
request with exactly one Response whose payload is one JSON object
with exactly four members — `volumes_disk`, `images_disk`,
`container_memory` and `container_disk`, each an integer no less than
`0` and no greater than `18446744073709551615` — with no tag and no
length prefix, followed by the Response Finish. The Provider shall
answer every `capacity` request whatever its `capabilities` object
holds out, shall not send a member this revision does not define,
and shall not send an error or a refusal on this Endpoint.

(b) **The meaning.** Each member shall state, in bytes, the most of
its resource — the size of a new Volume, for `volumes_disk`; what a
build could add to the Client Image Store, for `images_disk`; the
`memory` a container run could reserve, for `container_memory`; the
`disk` a container run could reserve, for `container_disk` — that the
Identity of the Connection could use as of the instant the Provider
sends the Response, as the Provider allots it. A Provider that
allots the Identity less of a resource than the Provider holds shall
report the Identity's allotment, and a Provider that allots the
Identity the whole of a resource shall report the whole; a Provider
shall report no more of a resource than it would, at that instant,
let the Identity use. The Provider shall reckon `container_memory`
and `container_disk` against what its running Containers reserved,
by the `memory` and `disk` of their requests, and not against the
memory or the disk free on its host.

(c) **Nothing reserved.** The Provider sets nothing aside by
answering. A request that relies on a member is judged by the
Obligation that governs it, and the Provider may refuse it for want
of room after a Response that reported room for it.

(d) **Members not held out.** Where the Provider holds out no mode
object, `volumes_disk` carries no obligation under (b); where the
Provider does not hold out `images_store`, `images_disk` carries no
obligation under (b). The Provider shall send each member
nonetheless.

### 5.7 `images::check` (tag 14)

The Provider shall answer every `images::check` request with exactly
one Response and the Response Finish. The sources the Provider has
are `client`, where it holds out the `images_store` Capability, and
`registries`, where its `images_registries` is not empty; the
Provider shall answer the byte `2` alone when the request asks for no
source the Provider has — neither source, or only sources it has
not — and then the Provider shall look nowhere. Otherwise the
Provider shall answer the byte `0` followed by exactly
`{"type":"available"}` when, at the time of the answer, at least one
source the request asks for and the Provider has holds the image the
digest names — `client` being the Client Images of the Identity and
no other Identity's, and `registries` the registries in the
Provider's `images_registries` that a reference of the request names,
asked by each reference's name — for a container request naming it;
the byte `0` followed by exactly `{"type":"unavailable"}` when no such
source has it; or the byte `1` followed by an error when it did not
determine the answer. The Provider shall not answer available on
account of a source the request did not ask for or the Provider has
not, shall not answer available on account of a Client Image of
another Identity, shall not consult a registry its
`images_registries` does not list, and shall not answer available on
account of an image it holds on its own account, which no request
can ask for. An available answer reserves nothing.

### 5.8 Volumes

Every covenant of this Section binds a Provider that holds out at
least one Volume mode object; a Provider that holds out none shall
answer every request of the eleven Volume Endpoints as Section 5.5(d)
provides and is bound by nothing else in this Section.

(a) **Listing.** The Provider shall answer every `volumes::list`
request with a Scope kept open: first, for every Volume available to
the Identity when the Scope opened, exactly one Response, the byte `0`
followed by the Volume in the postcard form the Specification states;
then exactly one Response whose payload is the byte `3` and nothing
else, sent once on the Scope; then, for the life of the Scope, a
Volume that becomes available to the Identity as the byte `0`
followed by the Volume, a Volume whose `bytes` or `mode` changes as
the byte `1` followed by the Volume, and a Volume that ceases to be
available as the byte `2` followed by the Volume as the Provider last
sent it, each sent after the Provider has sent the byte `0` of the
request that made the change; and the Response Finish when the
Provider receives the Identity's `stop` Channel request or the
Identity's Connection ends, or the byte `5` followed by an error and
the Response Finish when the Provider cannot list. The listing shall
contain every Volume the Identity created by `volumes::create` and
has not deleted by `volumes::delete`; it may contain other Volumes at
the Provider's discretion, reported as the Provider learns of them;
every Volume it contains shall be available to the Identity to mount
as the mode object for its mode allows; it shall contain no Volume of
a mode the Provider holds out no mode object for; no two Volumes
listed at once shall share a name. The Provider shall send a Volume
as added exactly once while it is listed, as changed only while it is
listed and only when its `bytes` or `mode` differ from what the
Provider last sent for it, and as removed exactly once and only after
it sent the Volume as added; shall report for each Volume its `name`,
its `bytes`, its `created` and its `mode` as the Specification
defines them, and nothing more; and shall not end the Scope because
the Identity has no Volume.

(b) **The hold of an operation.** For every `volumes::stat`,
`volumes::read`, `volumes::write`, `volumes::filetree`,
`volumes::edit` and `volumes::delete` request naming a Volume in the
Identity's listing, the Provider shall hold the Volume to itself from
the moment it accepts the request until the Response Finish, a
deletion never giving the hold back; shall refuse every run, every
FUSE serve and every other operation naming the Volume for as long
as it holds it; and shall answer a request naming a Volume that is
held at the time of the request with exactly one Response, alone,
and the Response Finish, deciding in the order the Specification's
Capabilities layer states for an operation: the Endpoint's `Held`
byte where another operation holds the Volume, its `Mounted` byte
where a running Container holds it, its `FuseServed` byte where a
FUSE serve holds it — `1`, `2` and `3` respectively for `Mounted`,
`FuseServed` and `Held` on `volumes::stat`, `volumes::read`,
`volumes::write`, `volumes::filetree` and `volumes::delete`, and
`4`, `5` and `6` on `volumes::edit` — changing nothing and, for a
write, asking for no content. The Provider shall not answer a hold
with an error.

(c) **Stat.** The Provider shall answer every `volumes::stat` request
naming a Volume in the Identity's listing that nothing holds with
exactly one Response carrying the Volume's `name`, `bytes`, `created`
and `mode`, its `bytes_used`, and its `dirhash` as the Specification
defines each, as of the time of the Response, after the byte `0`; and
shall answer a name not in the listing with the byte `5` followed by
an error.

(d) **Reading.** The Provider shall answer every `volumes::read`
request naming a Volume in the Identity's listing that nothing holds
and a path of one file in it with the bytes of that file as Responses
whose first byte is `0`, each of at most `CHUNK_SIZE` bytes, in order,
and the Response Finish, a file of zero bytes being exactly one such
Response of zero bytes; or with the Responses sent so far, exactly one
Response whose first byte is `5` followed by an error, and the
Response Finish. The Provider shall answer a name not in the listing,
a path that is not of the form the Specification states, and a path
that names no regular file, with the byte `5` followed by an error.

(e) **Writing.** The Provider shall answer every `volumes::write`
request naming a Volume in the Identity's listing that nothing holds
and a path of one file in it by creating every missing directory
above the file, opening exactly one Channel for the content, writing
the content the Identity sends on it, putting the file in place
whole, and answering with exactly one Response, the byte `0` only
after the file is at the path, or the byte `5` followed by an error,
and the Response Finish. The Provider shall leave at the destination
either what was there or the whole new file, and never a part of the
new file, and shall leave the destination as it was on any failure, a
content Channel that ends in an error included. From the moment the
Provider sends the byte `0`, every `volumes::stat` Response the
Provider sends the Identity shall report the Volume with the file in
place. The Provider shall answer a name not in the listing and a path
that is not of the form the Specification states with the byte `5`
followed by an error, opening no Channel.

(f) **Filetree.** The Provider shall answer every `volumes::filetree`
request naming a Volume in the Identity's listing that nothing holds
and a path of a directory in it, the root included, with exactly one
Response: the byte `0` followed by the entries of that directory, each
with everything beneath it, as the nodes the Specification's filetree
defines, every `Directory` with `changes` `0`; or the byte `5`
followed by an error. The Provider shall send no change after the
Response, and shall answer a name not in the listing, a path that is
not of the form the Specification states, and a path that names no
directory, with the byte `5` followed by an error.

(g) **FUSE serving.** The Provider shall answer every
`volumes::fuse_serve` request naming a Volume in the Identity's
listing with exactly one Response, deciding in the order the
Specification's Capabilities layer states for a FUSE serve: the byte
`4` followed by the name of the Volume and the mode it is in as JSON,
when the request's `mode` is not the Volume's; the byte `5` alone,
when the Provider holds out no mode object for the Volume's mode or
one whose `fuse_serve` is `false`; the byte `3` alone, when an
operation holds the Volume; the byte `2` alone, when another FUSE
serve holds the Volume and the mode object's `fuse_serve_many` is
`false`; the byte `1` alone, when a running Container holds the
Volume and the mode object's `mount_and_fuse_serve` is `false`; the
byte `0` alone, only after it holds the Volume as a running Container
that mounts it holds it; or the byte `6` followed by an error — a
name not in the listing, an `ephemeral` Volume whose `overlay_disk`
the Provider cannot set aside, or a Volume it could not open. Every
refusal changes nothing and holds nothing. The Provider shall hold
the Volume from the byte `0` until the Response Finish; shall permit
as many FUSE serves and running Containers to hold the Volume at
once as the mode object for its mode allows and no more; shall answer
a `volumes::stat`, `volumes::read`, `volumes::write`,
`volumes::filetree`, `volumes::edit` or `volumes::delete` naming the
Volume with that Endpoint's `FuseServed` byte for as long as it holds
it. After the byte `0` the Provider shall answer every Channel the
Client opens on the Scope carrying one of the nine FUSE asks, in the
form the Specification states for a mount's ask of the same tag, with
exactly one Channel Response and the Channel Response Finish, the
answer being what the Specification defines for that ask, determined
from the Volume as it is at the time of the answer: a read of a piece
at an offset, a write of a piece in place before the answer `ok`, a
stat carrying the kind, size, mode, owner, group and times the Volume
records. The Provider shall buffer no content of the Volume on the
Client's behalf. For a `persistent` Volume the Provider shall make
every change it answers `ok` in the Volume. For an `ephemeral` Volume
the Provider shall set aside the request's `overlay_disk` bytes for
the FUSE serve's life, refusing the FUSE serve by the byte `6`
followed by an error when it cannot; shall make every change it
answers `ok` in a layer of that FUSE serve alone, which every later
ask of the same FUSE serve sees and no other FUSE serve, Container or
request sees; shall keep the layer within `overlay_disk` bytes,
answering a change that would need more with an error and leaving the
Volume and the layer as they were while the FUSE serve continues; and
shall discard the layer at the Response Finish, leaving the Volume as
it was before the FUSE serve. For a `read_only` Volume the Provider
shall answer every write, truncate, setattr, removal, rename and mkdir
with the byte `2` and shall not change the Volume, and shall answer
every stat, read and listing as for any Volume. On the Client's stop
the Provider shall answer every ask still open, send the Response
Finish, and release the Volume; on the close of the Connection it
shall release the Volume. The Provider shall send nothing on the
stop's Channel.

(h) **Capacity.** The Provider shall answer every
`volumes::create_capacity` request with the largest size in bytes of a
Volume the Identity could create at the time of the Response, and
every `volumes::edit_capacity` request naming a Volume in the
Identity's listing with the number of bytes by which that Volume's
size could be increased at the time of the Response, each as a
postcard varint after the byte `0`, or an error after the byte `2`.
Neither answer reserves anything.

(i) **Creation.** The Provider shall answer every `volumes::create`
request with exactly one Response: the byte `2` alone when it holds
out no mode object for the request's `mode`, before anything is
reserved; the byte `0` only after a Volume of the stated name, size
and mode exists for the Identity; the byte `1` when the Provider
cannot reserve the size stated; or the byte `4` followed by an error
for any other reason. From the moment the Provider sends the byte
`0`, every listing the Provider opens for the Identity shall contain
the Volume, and every listing it holds open for the Identity shall
tell the Volume as added, until it is deleted. A Volume shall be
created empty, and the Provider shall not write to, remove from, or
otherwise alter the content of a Volume on its own account: only a
Container in which the Identity mounts the Volume changes its
content. The Provider shall refuse, by the byte `4`, a creation whose
name is already in the Identity's listing.

(j) **Editing.** The Provider shall answer every `volumes::edit`
request naming a Volume in the Identity's listing with exactly one
Response: the byte `3` alone when the request's change states a mode,
as `Mode` or as `Both`, for which the Provider holds out no mode
object, before the Volume is held; a hold as Section 5.8(b) provides;
the byte `0` only after the Volume has what the request's change
states, its size, its mode, or both; the byte `1` when the Provider
cannot reserve the size stated; the byte `2` when the content of the
Volume exceeds the size stated; or the byte `8` followed by an error
for any other reason. A change that states a size and a mode is one
edit: the Provider shall not change the mode of a Volume whose size
it refuses. The size and the mode are the only properties an edit
changes. From the moment the Provider sends the byte `0`, every
listing the Provider opens for the Identity and every stat it sends
the Identity shall report the new size and the new mode, every
listing it holds open for the Identity shall tell the Volume as
changed, and every Container in which the Identity mounts the Volume
after that moment, and every FUSE serve of it after that moment,
shall be bound under the new mode.

(k) **Deletion.** The Provider shall answer every `volumes::delete`
request naming a Volume in the Identity's listing with exactly one
Response: a hold as Section 5.8(b) provides; the byte `0` only after
the Volume no longer exists; or the byte `5` followed by an error for
any other reason. The Provider shall not delete a Volume that
anything holds at the time of the request. From the moment the
Provider sends the byte `0`, every listing the Provider opens for the
Identity shall omit the Volume, every listing it holds open for the
Identity that told the Volume as added shall tell it as removed, and
no listing shall tell as available a Volume that has been deleted. A
refusal and an error shall leave the Volume, its content and the
listing as they were.

(l) **Identity of the Volume namespace.** The Provider shall resolve
every Volume name against the Identity of the Connection on which it
is named and against no other. A Client shall not be able to name,
mount, stat, read, write, walk, FUSE serve, edit or delete a Volume
of another Identity.

### 5.9 Client Images

Every covenant of this Section but (f) binds a Provider that holds out
the `images_store` Capability; a Provider that does not shall answer
every `images::list`, `images::stat`, `images::build`, `images::delete`
and `images::oci` request as Section 5.5(d) provides, holds no Client
Image for any Identity, and is bound by nothing else in this Section.

(a) **Listing.** The Provider shall answer every `images::list`
request with a Scope kept open: first, for every Client Image of the
Identity when the Scope opened, exactly one Response, the byte `0`
followed by the image in the postcard form the Specification states;
then exactly one Response, the byte `2` alone; then, for the life of
the Scope, exactly one Response of the byte `0` followed by each
Client Image the Identity builds, and exactly one Response of the
byte `1` followed by each Client Image the Identity deletes, each
after the Provider has answered the build or the deletion; and the
Response Finish when the Identity's stop arrives, the Connection
ends, or exactly one Response of the byte `4` followed by an error
has been sent. The Provider shall not end the Scope because the
Identity has no Client Image, and shall send no Response of the byte
`1` before the byte `2`.

(b) **Stat.** The Provider shall answer every `images::stat` request
naming a Client Image of the Identity with exactly one Response and
the Response Finish: the byte `0` followed by the image as the
listing reports it and the Containerfile it was built from,
verbatim, in the postcard form the Specification states; or the byte
`2` followed by an error. It shall answer a request naming no Client
Image of the Identity with the byte `2` followed by an error.

(c) **Build.** The Provider shall answer every `images::build`
request with zero or more Responses of the byte `0`, each followed by
one line of the build's output as a string, in the order the build's
tool wrote them; then exactly one Response and the Response Finish:
the byte `1` followed by the digest and the size of the image only
after a Client Image of the stated name exists for the Identity; the
byte `2` alone when the Provider's store has no room for the image,
and then nothing exists; the byte `3` alone, with no output before
it, when the Volume the request names as its context is held as the
Specification's Capabilities layer states for a build's context, and
then nothing exists; or the byte `5` followed by an error for any
other reason, and then nothing exists. From the moment the Provider
sends the byte `1`, every listing the Provider opens for the Identity
shall contain the image, and every listing it holds open for the
Identity shall tell the image as added, until it is deleted. The
Provider shall refuse, by the byte `5`, a build whose name is already
in the Identity's listing. Where the request names a context, the
Provider shall resolve the Volume named against the Identity, shall
hold it for the duration of the build as a running Container that
mounts it holds it, under the mode object the Provider holds out for
the Volume's mode, shall read the directory named and write nothing
into it, and shall refuse the build by the byte `5` where the Volume
is not in the Identity's listing or where the path is not one of
names, and by the byte `3` where the hold cannot be taken. The
Provider shall not remove a Client Image on its own account: a Client
Image exists until the Identity deletes it.

(d) **Deletion.** The Provider shall answer every `images::delete`
request naming a Client Image of the Identity with exactly one
Response and the Response Finish: the byte `0` only after the image
no longer exists; the byte `1` when a running Container of the
Identity runs the image at the time of the request, and then nothing
changed; or the byte `3` followed by an error for any other reason.
From the moment the Provider sends the byte `0`, every listing the
Provider opens for the Identity shall omit the image, and every
listing it holds open for the Identity that told the image as added
shall tell it as removed. An error shall leave the image and the
listing as they were.

(e) **Identity of the image namespace.** The Provider shall resolve
every Client Image name against the Identity of the Connection on
which it is named and against no other. A Client shall not be able
to list, examine, build over or delete a Client Image of another
Identity. The Provider shall treat a Client Image as an image it
holds for the Identity that built it — an `images::check` by that
Identity naming its digest is available under Section 5.7, and a
container request by that Identity naming its digest runs it from
what the Provider holds under Section 5.10(b) — and shall not supply
a Client Image to any other Identity under either Section, unless
that Identity has itself built an image of the same digest.

(f) **Pull.** The Provider shall, where it holds out the
`images_store` Capability, serve every `images::oci` request
as a Scope kept open on which it sends no Response, and shall answer
every Channel the Identity opens on it from what it holds for the
Identity — the Client Images of the Identity, each byte for byte as
it was at the end of its build, and no other image, neither another
Identity's Client Image nor an image the Provider holds on its own
account: an `oci-has` Channel by exactly one Channel Response, the
byte `1` followed by a repository path for an image it holds for the
Identity and the byte `0` otherwise, and the Channel Response Finish;
an `oci-manifest` Channel by at most one Channel Response, the media
type and the manifest the digest names verbatim, and the Channel
Response Finish, the Finish alone for a digest it does not hold for
the Identity; an `oci-blob` Channel by the blob the digest names in
zero or more Channel Responses, each a piece in order, and the
Channel Response Finish, the Finish alone for a digest it does not
hold for the Identity; and a `disconnect` Channel by nothing on that
Channel. The Provider shall obtain nothing for a pull. It shall
answer a Channel Request that does not decode by a Bare Finish. It
shall end the Scope by the Response Finish, after the last answer of
every Channel still open, when it receives the Identity's
`disconnect` and when the Identity's Connection ends, and shall send
the Response Finish with no Response before it, serving no Channel,
for a pull it will not serve. A Provider that does not hold out the
`images_store` Capability shall answer every `images::oci` request
with exactly one Response on the main stream, the byte `0` alone, and
the Response Finish, and shall answer no Channel on the Scope.

### 5.10 Container Deployment — `containers::agents::run` (tag 0) and `containers::tools::run` (tag 1)

For every container run request the Provider accepts, the Provider
shall perform the following, in the order stated, and shall not
perform a later step before the earlier step is complete:

(a) **Read and refuse.** The Provider shall read the request. It shall
answer a request whose payload does not decode by a Bare Finish. It
shall answer by exactly one Response, the error, and the Response
Finish, a request that names a Volume not in the Identity's listing;
a mount whose path is the root of the Container; two mounts with one
path; a mount inside another mount; two FUSE mounts with one
`id`; a mount path with a component that is empty, `.` or `..`; or an
image the Provider will not supply. It shall then decide, for each distinct
Volume the request names in `volume_mounts`, in the order first
named and in the order the Specification's Capabilities layer states
for a run's Volume Mount, the first rule that applies deciding, and
shall answer by exactly one Response and the Response Finish — the
byte `5` followed by the name of the Volume and the mode it is in as
JSON, where the Volume's mode is not the `mode` of every mount naming
it; the byte `4` followed by the name as JSON, where the Provider
holds out no mode object for the Volume's mode or one whose `mount`
is `false`; the byte `1` followed by the name as JSON, where an
operation holds the Volume; the byte `2` followed by the name as
JSON, where a running Container holds the Volume and the mode
object's `mount_many` is `false`; the byte `3` followed by the name
as JSON, where a FUSE serve holds the Volume and the mode object's
`mount_and_fuse_serve` is `false` — releasing every Volume held for
the request, fetching nothing, deploying nothing and changing no
Volume's mode; and otherwise shall hold the Volume, as a running
Container holds it, from that moment until the run ends. A request
that names one Volume more than once mounts it at each path named
and is one running Container holding it. The Provider shall not
answer any refusal of this paragraph with an error.

(b) **Obtain the image.** The Provider shall obtain the image the
digest names, from what it holds — among which every Client Image of
the Identity, and no Client Image of another Identity — from a
referenced registry its policy lists, or from the Client, as it
chooses; the Specification
does not prescribe which, and a reference naming a registry the
Provider's policy does not list binds the Provider to nothing. Where
the Provider would obtain the image from the Client, it may ask the
Client, on the `oci-has` Channel, whether the Client holds the image,
and, on the `oci-manifest` and `oci-blob` Channels, for each manifest
and blob it requires; a Client that finishes an `oci-has` Channel
with no frame, or with the byte `0`, holds nothing under the digest;
a Client that answers the byte `1` followed by a repository path
holds the image under that path, from which the Provider may fetch
it through its own registry; and a Client that finishes an
`oci-manifest` or `oci-blob` Channel with no frame does not hold that
digest. The Provider shall run only the image the
digest names: bytes that are not those the digest names, and an image
the Provider obtains from no source, are the run's error. How the
Provider holds, verifies or streams what it obtains is not
prescribed.

(c) **Deploy.** The Provider shall perform Container Deployment as
Section 1.12 defines it, with `memory` and `disk` of the request as
ceilings; every Volume Mount resolved by `volume_name` against the
Identity, descended by `volume_relative_path`, and made present at
`container_path`, the Container's changes to it being in the Volume
when the Container ends if the Volume's mode is `persistent`, the
Volume being as it was before the run when the Container ends if the
Volume's mode is `ephemeral`, and every change refused inside the
Container if the Volume's mode is `read_only`; the Container Proxy
placed inside
and started; and
TCP port 14979 reachable to the Provider's Server. The Provider shall
set no environment variable in the Container from the request, and
shall expose no port of the Container other than port 14979. A failed
Deployment is the run's error.

(d) **Connect to the Proxy.** The Provider shall open exactly one
WebSocket connection to TCP port 14979 of the Container and, on it
before any other Scope, the begin Scope of the Container's family,
carrying the request's `arguments` and `template` verbatim and no
secret, and shall await its answer. A Proxy that does not accept the
connection, or that answers the begin Scope with an error, is a
Container that did not come up: the Provider shall stop the Container
and treat the failure as the run's error. The Provider shall open no
second connection to a Container.

(e) **Make every FUSE Mount.** For each entry of `fuse_file_mounts`,
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
restriction of its own on a FUSE Mount. Every read and every write a
program performs on a FUSE Mount shall be relayed as one ask carrying
that read's offset and length, or that write's offset and bytes, as
it arrives; the Provider shall buffer no content of a FUSE Mount on
the Container's behalf, shall report every attribute of an entry as
the Client's answer states it, and shall report a write, truncate,
setattr, removal, rename or mkdir the Client answers with the
read-only byte to the program as a read-only filesystem.

(f) **Hold what the Client opened.** The Provider shall serve a
Channel the Client opened before the id was sent only after the id is
sent, in the order the Channels were opened, and shall neither refuse
nor read such a Channel before the last FUSE Mount is complete.

(g) **Carry the arguments and the template.** The Provider shall
carry the request's `arguments` and `template` verbatim in the begin
Scope's request, and in nothing else, and shall read the `template`
for nothing. The request carries no secret, and the Provider shall
hold none: which secrets a Container takes is its program's answer
at its registration, and their values are the Client's, carried
under paragraphs (h) and (i). A begin the Proxy answers with an
error is the run's error, and the Provider shall stop the Container.

(h) **Register the Container.** On every run Scope of either family,
the Provider shall choose the id of (i) before this step and shall
enter the Container among those it runs, so that a
`containers::fuse_serve` request naming the id is answered from this
step on; the Provider shall list an agent Container to nobody. When
the Proxy's answer to the begin Scope carries one or more dependency
tool templates or one or more declared secrets, the Provider shall
open exactly one `registration` Channel on the run Scope carrying the
id, the templates and the declaration verbatim — every template
whole, its image, limits, arguments, type, scope, mounts, secrets and
permissions unread; every declared secret's name and mode unread —
before the id is sent and beside the FUSE Mounts, and shall read the
Client's answer to its finish. Where the answer is the byte `0`
followed by the secrets' values, the Provider shall open exactly one
`secrets` Channel on the begin Scope carrying those values verbatim
and unread, and shall read the Proxy's answer to its finish; the
Proxy's `Fed` lets the run go on. The Provider shall treat the byte
`1` on either Channel, a Bare Finish on either, and a Client that is
gone as the run's error and shall stop the Container. The Provider
shall open no `registration` Channel and feed nothing when the
templates and the declaration are both empty. On a
`containers::tools::run` Scope the templates are always empty — a tool
Container declares no dependencies, and its begin's answer carries
none — and the `registration` Channel carries the declaration alone.

(i) **Send the id.** The id the Provider chose in (h) shall be unique
among the Containers it is running and not derivable from the
request, from the Identity, or from any other id, and the Provider
shall send it as exactly one Response, the same id the `registration`
Channel carried, and only after that Channel has ended and its feed,
where there was one, has been answered `Fed`; a run refused under
paragraph (a) has the refusal there stated — the byte `1`, `2`, `3`,
`4` or `5` followed by the name of the Volume — as its only Response,
and a run that fails before the id has the error, the byte `6`
followed by it, as its only Response. From that moment the Container
is running for the Client and shall be findable by a
`containers::fuse_serve` request of the Client's naming the id. After
the id the Provider shall send, for a tool container, the byte `7`
whenever the Proxy sends `Active` on the begin Scope's main stream and
the byte `8` followed by the values verbatim whenever the Proxy sends
`Inactive`, and no other Response; and, for an agent container, every
chunk the Proxy sends on the begin Scope's main stream as one
Response, the byte `7` followed by the chunk verbatim, in the order
the Proxy sent them, bracketed by the byte `8` before each loop and
the byte `9` followed by the values verbatim after it, and no other
Response; save that, where the Proxy ends the begin Scope with an
error after `Begun`, the Provider shall send that error as one
Response, the byte `6` followed by it verbatim, and the Response
Finish after it. The Provider shall relay every `secrets` Channel the
Client opens on the run Scope as exactly one `secrets` Channel on the
begin Scope, the values verbatim and unread, and shall answer the
Client's Channel as the Proxy answers the Provider's.

(j) **Serve the Scope.** For as long as the Scope lives, and
concurrently, the Provider shall relay every Channel the Proxy opens
on the begin Scope, and every ask a FUSE Mount makes on its Scope, to
the Client as a Channel the Provider opens, in the form the
Specification states for that ask, and shall serve every Channel the
Client opens as the Specification states for that Channel.

(k) **Make a FUSE Mount while the Container runs.** The Provider
shall serve every `fuse_mount` Channel the Client opens on a run
Scope with exactly one Channel Response and the Channel Response
Finish: the byte `2` where it holds a FUSE Mount under the request's
id, and the byte `1` where the request's path is a mount's the
Container holds, lies inside one or holds one — a Volume Mount's or
a FUSE Mount's, the request's or one made since — each decided in
that order, before the Proxy is asked and without asking it; else
the byte `0`, only after the Proxy has answered its mount Scope
`Ok`, or the byte `3` followed by the Proxy's error where the Proxy
refused the mount. For a request it does not refuse the Provider
shall open exactly one mount Scope on the Proxy's connection, naming
the path and the kind, shall wait for its answer, and shall send the
byte `0` before it relays any ask of the mount. The Provider shall
hold the request's id and path from the moment it decides to ask the
Proxy, so that two requests it cannot both make are not both made,
and shall let both go where the Proxy refuses. A Proxy connection
that ends before the Proxy answers is a Bare Finish on the Channel.
A request whose path is the root, or has a component that is not a
name, is a request that did not decode. A mount made this way is a
FUSE Mount of the Container for every other Obligation of this
Section and of Section 5.11, lives until the run ends, and is
unmounted by nothing; every tree Scope the Provider opens after the
mount shall name the mount's path among those the tree leaves out.

(l) **End the run.** When the Provider receives the Client's stop
channel request, when the Proxy's connection ends, or when the
Client's Connection ends, the Provider shall stop the Container, end
every Channel task, and send the Response Finish with no error. After the
id, the Provider shall send no error of its own on the main stream of
the Scope; the one error it sends after the id is the Proxy's, under
paragraph (i).

### 5.11 The Conduit

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

(f) For a filetree the Client opens, the Provider shall serve a tree
that leaves out every FUSE Mount of the Container and contains every
Volume Mount of the Container, shall report every change under a
Volume Mount as the Specification's filetree states one, and shall
stop watching when the Client's Scope ends. How the Provider watches
a Volume Mount is not prescribed.

(g) For a transfer the Client opens, the Provider shall serve it only
when the Client is the Runner of both the Container of the Scope and
the Container the request names, and shall
otherwise answer exactly one Channel Response, the error
`{"kind":"denied"}`, and the Channel Response Finish, without
distinguishing a Container the Client may not reach from an id under
which no Container is running. When served, the Provider shall open a
read Scope on the Proxy of the Container of the Scope for the stated
path and a write Scope on the Proxy of the named Container for the
stated destination, relay the bytes of the one into the other, and
answer the Client's Channel with exactly one Channel Response,
transferred or an error, after the named Container's Proxy has
answered. The Provider shall send nothing of the file to the Client.

### 5.12 `containers::fuse_serve` (tag 2)

For every FUSE serve request, the Provider shall, in order: (a) read
the request and answer a payload that does not decode by a Bare
Finish; (b) look the `id` up among the Containers it is running whose Runner
is the Identity of the Connection, and answer an id that names none —
no Container at all, or one the Identity does not run — by exactly
one Response, the byte `1`
followed by exactly `{"kind":"missing"}`, and the Response Finish;
(c) open exactly one `filesystem::fuse_serve` Scope on the Container's
Proxy carrying the request's `path` verbatim, and answer the Proxy's
error by exactly one Response, the byte `1` followed by that error
verbatim, and the Response Finish, and a Proxy that cannot be reached
by exactly one Response, the byte `1` followed by an error, and the
Response Finish; (d) answer exactly one Response, the byte `0` and
nothing else, only after the Proxy has answered its byte `0`; (e) for
the life of the Scope, relay every Channel the Client opens on it —
each of the nine asks, and each filetree — as exactly one Channel on
the `filesystem::fuse_serve` Scope carrying the bytes of the Client's
Channel Request verbatim, and relay every Channel Response of the
Proxy on that Channel as a Channel Response on the Client's Channel,
byte for byte, in order, and the Proxy's Channel Response Finish as
the Client's Channel Response Finish, answering a Channel Request that
does not decode by a Bare Finish; (f) end the Scope by the Response
Finish, after the last answer, when it receives the Client's `stop`
Channel request, when the Client's Connection ends, and when the run
of the Container ends, stopping the `filesystem::fuse_serve` Scope on
the Proxy and holding and releasing nothing of the Container. The
Provider shall serve a Container of either family alike, shall serve
any number of FUSE serve Scopes of one Container at once, shall hold
no Volume and no mount on account of a FUSE serve, shall time nothing
out, and shall send nothing on the main stream of a FUSE serve Scope
after the byte `0`.

### 5.13 Daemon connections — `daemons::accept` (tag 22) and `daemons::connect` (tag 23)

(a) For every accept request, the Provider shall, in order: read the
request and answer a payload that does not decode by a Bare Finish;
answer an accept Scope already open under the Identity of the
Connection, on any Connection, by exactly one Response, the byte `1`
followed by exactly `{"kind":"held"}`, and the Response Finish; hold
the Identity for the Scope; answer exactly one Response, the byte `0`
followed by the Identity as JSON; and hold the Scope open until the
Acceptor's `stop` Channel request or the Acceptor's Connection ending,
on which it shall finish every half of every connection open on the
Scope, finish every connect Scope relayed to it, release the Identity,
and send the Response Finish.

(b) For every connect request, the Provider shall, in order: read the
request and answer a payload that does not decode by a Bare Finish;
answer a `daemon` under which no accept Scope is open by exactly one
Response, the byte `1` followed by exactly `{"kind":"missing"}`, and
the Response Finish; mint a connection id unique among the
connections announced on that accept Scope and not yet finished, and
open on the accept Scope exactly one `connection` Channel carrying as
JSON the id, the Identity and peer IP address of the Daemon
Connector's Connection as the Provider observed them, and the
request's `mode` verbatim; answer the Acceptor finishing that Channel
with no Channel Response before the finish, and the accept Scope
ending before the Acceptor has opened its half, by exactly one
Response, the byte `1` followed by exactly `{"kind":"denied"}`, and
the Response Finish; and, when the Acceptor opens its half quoting
the id, send exactly one Response, the byte `0` and nothing else,
before any frame of the Acceptor's, and open on the connect Scope
exactly one `frames` Channel carrying nothing.

(c) For every connection opened, the Provider shall relay every
Channel Response of the `frames` Channel onto the Acceptor's half as
exactly one Channel Response each, in order, and relay every Channel
Response the Acceptor sends on the Provider's half onto the connect
Scope as exactly one Response each, the byte `2` followed by the
frame verbatim, in order; shall hold a Channel Response the Acceptor
sends before it has opened its half and relay it after the byte `0`,
in order; shall read, alter, reorder and repeat no frame; and shall
relay the `mode` without reading it.

(d) The Provider shall end a connection — finishing the Acceptor's
half, finishing the `frames` Channel, and sending the Response Finish
of the connect Scope with no error — when the Acceptor finishes the
Provider's half, when the Daemon Connector finishes the `frames`
Channel or answers on it a Channel Response that is not one client
frame of the daemon protocol, when the accept Scope ends, and when the
Daemon Connector's Connection ends. The Provider shall finish the
other half of a connection whose one half has finished.

(e) The Provider shall hold at most one accept Scope under one
Identity at a time, shall time nothing out, and shall send nothing on
the main stream of a connect Scope after the byte `0` but the frames
of the Acceptor's and the Response Finish.

### 5.14 Prohibitions

The Provider shall never: (a) refuse, alter or withhold a FUSE ask on
its own account, whether a change to a FUSE Mount is allowed being the
Client's answer to that ask, and whether a change to a served
Container is made being the Proxy's answer to that ask; (b) write to
a Volume, a mount or a Container on its own account, a `volumes::write`
request of the Identity, a write a `volumes::fuse_serve` ask of the
Identity directs, a `write` the Identity opens on a run, and a write
a `containers::fuse_serve` ask of the Identity directs, being
on the Identity's account; (c) send a second id on a run Scope; (d) send an error on
the main stream of a run Scope after the id; (e) retry any ask; (f)
read, inspect, parse, log the content of, or act upon the content of
a Relayed Exchange, save to the extent necessary to relay it; (g)
impose a timeout on any fetch, Deployment, Channel, Scope or
Connection; (h) send a Response where the Specification states a Bare
Finish, or a Bare Finish where the Specification states a Response;
(i) mint a container id or a connection id that is derivable from
anything a Client chose; or (j) serve a Scope under an Identity other
than that of the Connection on which the Scope was opened.

### 5.15 The Container Proxy

The Provider shall place inside every Container a Container Proxy
that conforms in full to the Specification's Container Proxy and
Container Proxy Endpoints layers:
that listens on TCP port 14979 and accepts one WebSocket connection
there; that speaks the Protocol's own frames on it and sends no auth
frame; that answers exactly one begin Scope per connection, holding the
arguments and the template id it carries for the Container's life
and answering, for an agent Container, with the dependency tool
templates and the secrets the program declared, unread, and, for a
tool Container, with the secrets the program declared, unread, and no
dependencies; that hands the values of every `secrets` Channel to the
program as they are, unread, and answers `Fed` only once the program
holds them; that, from `Begun` and from every `Inactive`, delivers
nothing to a program that declared a secret — no message, no MCP
exchange — until the next `secrets` Channel is answered `Fed`; that
collects from the program the value of every `rotating` secret it
declared at the end of every loop of an agent Container and at the
end of the last MCP exchange in flight of a tool Container, and
carries them on the `Inactive`; that, where it cannot collect every
such value, ends the begin Scope with an error naming the secret and
carrying no value; that marks a tool Container's activity with
`Active` before the first exchange in flight is delivered and
`Inactive` after the last is answered; that opens no daemon Channel for an agent Container
and no MCP Channel outward for a tool Container, answering the
program of either that it has no such thing; that sets nothing under
`_meta` on any MCP exchange it relays, in either direction, or on
any chunk of an agent's conversation, and alters nothing of them;
that makes each
mount Scope's mount before answering it, whenever after the begin is
answered the Scope is opened, and holds every mount for its life,
asking for what the mount needs on channels of that Scope; that
leaves out of every filetree the paths the tree Scope's request names
and `/proc`, `/sys` and `/dev`; that serves a directory of the
Container live on a `filesystem::fuse_serve` Scope, answering the nine
asks from the Container's filesystem, every change in place and none
refused on its own account, and every filetree of the Scope with the
directory's tree and every change in it, under `/proc`, `/sys` and
`/dev` nothing; that ignores text data frames; and that imposes no
timeout. Where the Provider uses a program other than
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
for use in a later run as the Specification permits; and (b) Volumes,
for as long as the Specification requires. The
Provider shall not retain any other Client Content after the Scope
that carried it ends, and shall not retain the content of a Relayed
Exchange at any time except transiently in the course of relaying it.

6.3 **No inspection.** The Provider's Obligation under Section 5.11(a)
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
places inside a Container conforms to Section 5.15; (d) its
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
published at `https://provider.diverge.network/2.3.0/` on the
Effective Date, is attached to this Agreement as the Markdown
rendering the site publishes beside each page of the revision, the
pages being those the site's sitemap at
`https://provider.diverge.network/sitemap-index.xml` enumerates under
`/2.3.0/`, with every included Rust source file expanded in place, and
initialed by both Parties on each page.
