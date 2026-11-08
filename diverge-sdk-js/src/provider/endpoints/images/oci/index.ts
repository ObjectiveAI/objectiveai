// A provider's oci scope: whether it can supply one image, and, where
// it can, the OCI asks of it — a manifest by digest, a blob by digest,
// the proof — each on a channel the caller opens, and the cancel that
// ends the scope. What is here is the caller's half the daemon's
// `images.oci` shares: the channel requests, and the held scope's
// handle.

export * as client from "./client/index.js";
