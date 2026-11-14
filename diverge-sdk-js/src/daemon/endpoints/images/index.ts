// Images: named by digest, never held. `read` asks whether any source
// the daemon reaches can supply the image, and, where one can,
// everything read of it, through it: its manifests, its blobs, its
// proof, and its DIVERGE.md. `build` makes one on a provider, from a
// Containerfile, into the daemon's Store there.

export * as build from "./build/index.js";
export * as read from "./read/index.js";
