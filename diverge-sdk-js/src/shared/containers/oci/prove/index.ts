// Whether a party possesses an image: the nonce check. The ask is the
// platform manifest's digest, when the image's manifest is an index,
// and a fresh nonce; the answer is one frame, the 32 bytes of
// `SHA-256(nonce ‖ D)` — `D` the manifest, the platform's manifest
// where there is an index, then the config and every layer, in order,
// verbatim — then the finish, or the empty finish for an image not
// held. A digest is public; only the bytes answer a nonce never seen.

export * as request from "./request/index.js";
export * as response from "./response/index.js";
