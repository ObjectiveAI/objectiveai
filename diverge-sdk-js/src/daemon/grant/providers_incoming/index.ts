// Grants over incoming credentials.
//
// `Permission` is one grant's worth over incoming credentials, reaching by the
// family's own list filter; the actions are `shared.permission`'s.

export * from "./permission.js";
export { Make, Over } from "../../../shared/permission/providers_incoming/index.js";
