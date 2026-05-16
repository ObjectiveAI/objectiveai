import { checkScalarFields, checkVectorFields } from "../../wasm/loader.js";
import type { FunctionsCheckScalarFieldsValidation } from "./scalarFieldsValidation";
import type { FunctionsCheckVectorFieldsValidation } from "./vectorFieldsValidation";

export function wasmFunctionsCheckCheckScalarFields(fields: FunctionsCheckScalarFieldsValidation): void {
  checkScalarFields(fields);
}

export function wasmFunctionsCheckCheckVectorFields(fields: FunctionsCheckVectorFieldsValidation): void {
  checkVectorFields(fields);
}
