// Who a request came from, and who a response came from, as the
// daemon stamps them into `_meta`: a `Caller` on a request, a `Tool`
// on a response. A reader takes the one it expects with `request` or
// `response`, and an `AttestationError` is a `_meta` without one or
// with one that is not that.

export * from "./agent.js";
export * from "./caller.js";
export * from "./tool.js";
export * from "./error.js";
export * from "./request.js";
export * from "./response.js";
