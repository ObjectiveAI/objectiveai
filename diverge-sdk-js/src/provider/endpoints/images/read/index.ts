// A provider's read scope: whether it can supply one image, and, where
// it can, everything read of it — a manifest by digest, a blob by
// digest, the proof, and its DIVERGE.md — each on a channel the caller
// opens, and the cancel that ends the scope. What is here is the caller's half the daemon's
// `images.read` shares: the channel requests, and the held scope's
// handle.

export * as client from "./client/index.js";
