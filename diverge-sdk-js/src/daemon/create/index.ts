// What an agent's create and a tool's create share.
//
// An agent container and a tool container are made of the same
// things: a template, the secrets' vault keys, the provider they run
// on with its volumes, the FUSE mounts of providers' volumes. `Inner`
// is those, written once, and each family's create extends it with
// the one thing that is the family's — the name — so that the two
// requests are the same shape member for member on the wire.

export * from "./inner.js";
