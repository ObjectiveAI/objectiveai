import {
  validateFunctionInput as wasmValidateFunctionInput,
  compileFunctionInputMaps as wasmCompileFunctionInputMaps,
  compileFunctionTasks as wasmCompileFunctionTasks,
  compileFunctionOutput as wasmCompileFunctionOutput,
  compileFunctionOutputLength as wasmCompileFunctionOutputLength,
  compileFunctionInputSplit as wasmCompileFunctionInputSplit,
  compileFunctionInputMerge as wasmCompileFunctionInputMerge,
} from "../wasm/loader.js";
import { Function } from "./function";
import { CompiledFunctionOutput, InputValue, TaskOutputs } from "./expression";
import { CompiledTasks } from "./task";

export function validateFunctionInput(
  function_: Function,
  input: InputValue,
): boolean | null {
  const result = wasmValidateFunctionInput(function_, input);
  return result === undefined ? null : result;
}

export function compileFunctionInputMaps(
  function_: Function,
  input: InputValue,
): InputValue[][] | null {
  const result = wasmCompileFunctionInputMaps(function_, input);
  return result === undefined ? null : result;
}

export function compileFunctionTasks(
  function_: Function,
  input: InputValue,
): CompiledTasks {
  return wasmCompileFunctionTasks(function_, input) as CompiledTasks;
}

export function compileFunctionOutput(
  function_: Function,
  input: InputValue,
  task_outputs: TaskOutputs,
): CompiledFunctionOutput {
  return wasmCompileFunctionOutput(
    function_,
    input,
    task_outputs,
  ) as CompiledFunctionOutput;
}

export function compileFunctionOutputLength(
  function_: Function,
  input: InputValue,
): number | null {
  const result = wasmCompileFunctionOutputLength(function_, input);
  return result === undefined ? null : result;
}

export function compileFunctionInputSplit(
  function_: Function,
  input: InputValue,
): InputValue[] | null {
  const result = wasmCompileFunctionInputSplit(function_, input);
  return result === undefined ? null : result;
}

export function compileFunctionInputMerge(
  function_: Function,
  inputs: InputValue[],
): InputValue | null {
  const result = wasmCompileFunctionInputMerge(function_, inputs);
  return result === undefined ? null : result;
}
