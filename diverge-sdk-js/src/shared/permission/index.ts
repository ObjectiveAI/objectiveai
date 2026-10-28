// Permissions that name nothing: the grants a dependency tool
// template asks for its tool, and the actions every grant of the
// daemon's is made of.
//
// A GRANT is one permission over one kind of thing the daemon holds,
// and on the wire it is one object with one member, named for the
// kind, whose value is the kind's permission: `{"agents":…}`. The
// shapes are the daemon's own — to make, over what exists, tagging,
// the one thing — with one restriction, which is what makes these
// shareable: a grant here reaches what exists by TAGS alone, `Within`
// being `"any"` or a `Tags` test, and over agents alone the words
// `"attached"` and `"never_active"`. Each kind's `Make` and `Over` live here, and the daemon's
// grants are built of them.

export * from "./grant.js";
export * from "./tags.js";
export * from "./tagging.js";
export * from "./within.js";

export * as accounts from "./accounts/index.js";
export * as agents from "./agents/index.js";
export * as agents_templates from "./agents_templates/index.js";
export * as daemons from "./daemons/index.js";
export * as dependency_tools from "./dependency_tools/index.js";
export * as events from "./events/index.js";
export * as images from "./images/index.js";
export * as postgres from "./postgres/index.js";
export * as providers from "./providers/index.js";
export * as roles from "./roles/index.js";
export * as standalone_tools from "./standalone_tools/index.js";
export * as standalone_tools_templates from "./standalone_tools_templates/index.js";
export * as vault from "./vault/index.js";
export * as volumes from "./volumes/index.js";
