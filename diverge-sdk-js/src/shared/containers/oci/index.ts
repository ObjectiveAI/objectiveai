// An image's bytes by digest, and the proof: three of the asks a
// provider and its caller exchange over one image — a manifest by
// digest, a blob by digest, and the nonce check over the whole — as
// far as the daemon's answers reach into them, which is the daemon's
// `images.read` scope, on which a caller asks them of whatever source
// the daemon found. The image is never in an ask: it is the one the
// scope carrying the ask names. No tags cross, no offsets, no errors:
// an ask not answered is the empty finish.

export * as blob from "./blob/index.js";
export * as manifest from "./manifest/index.js";
export * as prove from "./prove/index.js";
