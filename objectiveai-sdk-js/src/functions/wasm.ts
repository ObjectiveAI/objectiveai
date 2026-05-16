import {
  compileFunctionTasks,
  compileFunctionInputSplit,
  compileFunctionInputMerge,
  compileFunctionOutputLength,
  validateFunctionInput,
  alphaCheckLeafScalarFunction,
  alphaCheckLeafVectorFunction,
  alphaCheckBranchScalarFunction,
  alphaCheckBranchVectorFunction,
} from "../wasm/loader.js";
import type { FunctionsFunction } from "./function";
import type { FunctionsCompiledTask } from "./compiledTask";
import type { FunctionsAlphaScalarInlineFunction } from "./alpha_scalar/inlineFunction";
import type { FunctionsAlphaVectorInlineFunction } from "./alpha_vector/inlineFunction";
import type { FunctionsFullRemoteFunction } from "./fullRemoteFunction";

export function wasmFunctionsCompileFunctionTasks(fn: FunctionsFunction, input: unknown): (FunctionsCompiledTask | FunctionsCompiledTask[] | null)[] {
  const raw = JSON.parse(compileFunctionTasks(fn, input));
  return raw.map((item: any) => {
    if (item === null) return null;
    if ("One" in item) return item.One;
    if ("Many" in item) return item.Many;
    return item;
  });
}

export function wasmFunctionsCompileFunctionInputSplit(fn: FunctionsFunction, input: unknown): unknown[] | undefined {
  const result = compileFunctionInputSplit(fn, input);
  return result !== undefined ? JSON.parse(result) : undefined;
}

export function wasmFunctionsCompileFunctionInputMerge(fn: FunctionsFunction, input: unknown[]): unknown | undefined {
  const result = compileFunctionInputMerge(fn, input);
  return result !== undefined ? JSON.parse(result) : undefined;
}

export function wasmFunctionsCompileFunctionOutputLength(fn: FunctionsFunction, input: unknown): number | undefined {
  return compileFunctionOutputLength(fn, input);
}

export function wasmFunctionsValidateFunctionInput(fn: FunctionsFunction, input: unknown): boolean | undefined {
  return validateFunctionInput(fn, input);
}

export function wasmFunctionsAlphaCheckLeafScalarFunction(fn: FunctionsAlphaScalarInlineFunction): void {
  alphaCheckLeafScalarFunction(fn);
}

export function wasmFunctionsAlphaCheckLeafVectorFunction(fn: FunctionsAlphaVectorInlineFunction): void {
  alphaCheckLeafVectorFunction(fn);
}

export function wasmFunctionsAlphaCheckBranchScalarFunction(fn: FunctionsAlphaScalarInlineFunction, children?: Record<string, FunctionsFullRemoteFunction>): void {
  alphaCheckBranchScalarFunction(fn, children);
}

export function wasmFunctionsAlphaCheckBranchVectorFunction(fn: FunctionsAlphaVectorInlineFunction, children?: Record<string, FunctionsFullRemoteFunction>): void {
  alphaCheckBranchVectorFunction(fn, children);
}
