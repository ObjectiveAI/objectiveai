// A blob by digest: one request naming the digest, answered with the
// blob's bytes as any number of frames, each appending, then the
// finish; zero frames is a digest not held. A config and a layer are
// both blobs.

export * as request from "./request/index.js";
export * as response from "./response/index.js";
