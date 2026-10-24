// The caller half of the daemon protocol: performing every exchange.
//
// Every endpoint's `execute` performs its exchange rather than
// describing it — encodes the request, opens a scope on a `Handle`,
// reads what comes back out of the frame envelope, and hands back
// the daemon's answer typed. The shapes:
//
// | shape | here | hands back |
// |-------|------|------------|
// | one answer | `one_shot` | the response frame, whichever answer it is |
// | a stream | `stream` | an `ExecuteStream` of response frames, ended by the finish |
// | a stream with a cancel | `stream` and `cancel` | the stream, and a `Cancel` that ends it |
// | one answer with a cancel | `pending` | a `Pending` that waits, or cancels |
// | an upload | `upload` | the response frame, once every content channel the daemon opened is answered |
//
// THE ANSWER IS THE FRAME. A daemon response is one of several
// answers and one failure, and an executor returns the whole frame:
// `forbidden`, `not_found` and the daemon's own `error` are values a
// caller matches, since each is something the daemon said. An
// executor's error is the absence of an answer.

export * as one_shot from "./one_shot.js";
export * as stream from "./stream.js";
export * as pending from "./pending.js";
export * as upload from "./upload.js";
export * from "./cancel.js";
export * from "./error.js";
