// Grants: what a role allows, kind by kind.
//
// A GRANT is one permission over one kind of thing the daemon holds,
// and a role is a list of them, held by accounts. On the wire a grant
// is one object with one member, named for the kind, whose value is
// the kind's permission. Every kind's permission takes one of these
// shapes, and the shape is the logic: TO MAKE, a bare array of the
// kind's making actions; OVER WHAT EXISTS, `actions` and `within` —
// `"any"`, or the kind's own list filter read as a test; TAGGING,
// `actions`, `within` and `tags`; and OVER THE ONE THING, the
// database, the events and images, a bare array of actions. The actions are
// the shared ones, defined once in `shared.permission` and
// re-exported here.

export * from "./grant.js";
export { Tagging, within } from "../../shared/permission/index.js";
export type { Within } from "../../shared/permission/index.js";

export * as accounts from "./accounts/index.js";
export * as agents from "./agents/index.js";
export * as agents_templates from "./agents_templates/index.js";
export * as dependency_tools from "./dependency_tools/index.js";
export * as roles from "./roles/index.js";
export * as standalone_tools from "./standalone_tools/index.js";
export * as standalone_tools_templates from "./standalone_tools_templates/index.js";
export * as vault from "./vault/index.js";
export * as volumes from "./volumes/index.js";
export * as postgres from "../../shared/permission/postgres/index.js";
export * as providers from "../../shared/permission/providers/index.js";
export * as daemons from "../../shared/permission/daemons/index.js";
export * as events from "../../shared/permission/events/index.js";
export * as images from "../../shared/permission/images/index.js";
