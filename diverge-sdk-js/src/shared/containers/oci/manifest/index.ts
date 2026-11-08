// A manifest by digest: one request naming the digest, answered with
// one frame — the manifest's media type and its bytes — then the
// finish, or the empty finish for a digest not held. An index is a
// manifest like any other.

export * as request from "./request/index.js";
export * as response from "./response/index.js";
