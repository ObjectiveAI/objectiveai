import {
  compileFunctionOutput as wasmCompileFunctionOutput,
  compileFunctionTasks as wasmCompileFunctionTasks,
} from "#wasm-loader";
import { Function } from "./function";
import { CompiledFunctionOutput, InputValue, TaskOutputs } from "./expression";
import { CompiledTasks } from "./task";

export function compileFunctionTasks(
  function_: Function,
  input: InputValue
): CompiledTasks {
  return wasmCompileFunctionTasks(function_, input) as CompiledTasks;
}

export function compileFunctionOutput(
  function_: Function,
  input: InputValue,
  task_outputs: TaskOutputs
): CompiledFunctionOutput {
  return wasmCompileFunctionOutput(
    function_,
    input,
    task_outputs
  ) as CompiledFunctionOutput;
}
