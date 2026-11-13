// Images: named by digest, never held. `info` reads the image's
// DIVERGE.md, from a container of it running or from a provider that
// can obtain it; `oci` asks whether any source the daemon reaches can
// supply the image, and, where one can, its manifests, blobs and
// proof, through it; `build` makes one on a provider, from a
// Containerfile, into the daemon's Store there.

export * as build from "./build/index.js";
export * as info from "./info/index.js";
export * as oci from "./oci/index.js";
