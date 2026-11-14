// Whether the daemon can supply an image, and, where it can, the OCI
// asks of it on one scope: the provider's `images.read`, asked of the
// daemon. The request is the question — an image, and the remote
// daemons not to ask — and the first response answers it. The daemon
// holds nothing: it asks every connected provider and every remote
// daemon the account's `read` grants reach, all at once, and the first
// to answer held is the source, every other cancelled; none held is
// `not_found`. Where the answer is `held` the scope stays open, and
// every ask the caller opens on a channel — a manifest, a blob, the
// proof — is asked of the source and answered as it answered, byte
// for byte; an ask it fails is the empty finish. The caller's
// `cancel` ends it.

export * as client from "./client/index.js";
export * as server from "./server/index.js";
