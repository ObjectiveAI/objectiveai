// The shapes more than one endpoint is made of.
//
// `error` is the one failure every endpoint answers with. `filetree`
// is a live filesystem view, and the fold that keeps one current.
// `listing` is a list stream's vocabulary and the list a consumer
// keeps from it, with its fold. `relay` is the member every daemon
// request may carry; `canonical` the one form a value is hashed in;
// `permission` the grants that name nothing and the actions every
// grant is made of; `containers` the image and the dependencies a
// record embeds; `mcp` the attestation this SDK puts in an MCP
// `_meta`. They live here rather than beside
// whichever endpoint used them first because a shape defined twice
// is two shapes that agree until they do not, and the folds are
// exactly the kind of thing that would stop agreeing quietly.

export * as canonical from "./canonical/index.js";
export * as containers from "./containers/index.js";
export * as error from "./error.js";
export * as filetree from "./filetree/index.js";
export * as listing from "./listing/index.js";
export * as mcp from "./mcp/index.js";
export * as permission from "./permission/index.js";
export * as relay from "./relay.js";
