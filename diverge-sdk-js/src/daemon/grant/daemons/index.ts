// Grants over remote daemons' records.
//
// `Permission` is one grant's worth over the records, reaching by the
// family's own list filter; the actions are `shared.permission`'s. A relay
// is judged as `relay` over the record its first name is.

export * from "./permission.js";
export { Make, Over } from "../../../shared/permission/daemons/index.js";
