// Grants over providers' records.
//
// `Permission` is one grant's worth over the records. The providers
// list filter names no tags, so a grant reaches by a test of its own,
// `Reach`: the providers named by identity, those whose records carry
// some tags, or both. The same test is the `within` of the two grants
// that act through a provider, `providers_daemons` and `images`. The
// actions are `shared.permission`'s.

export * from "./permission.js";
export * from "./reach.js";
export { Make, Over } from "../../../shared/permission/providers/index.js";
