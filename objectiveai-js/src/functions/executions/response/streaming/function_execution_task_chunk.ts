import z from "zod";
import {
  FunctionExecutionChunk,
  FunctionExecutionChunkSchema,
} from "./function_execution_chunk";
import {
  TaskIndexSchema,
  TaskTaskIndexSchema,
  TaskTaskPathSchema,
  TaskSwissRoundSchema,
  TaskSwissPoolIndexSchema,
} from "../task";
import { TaskChunkSchema } from "./task_chunk";

export interface FunctionExecutionTaskChunk extends FunctionExecutionChunk {
  index: number;
  task_index: number;
  task_path: number[];
  swiss_round?: number;
  swiss_pool_index?: number;
}

export const FunctionExecutionTaskChunkSchema: z.ZodType<FunctionExecutionTaskChunk> =
  z
    .lazy(() =>
      FunctionExecutionChunkSchema.extend({
        index: TaskIndexSchema,
        task_index: TaskTaskIndexSchema,
        task_path: TaskTaskPathSchema,
        swiss_round: TaskSwissRoundSchema.optional(),
        swiss_pool_index: TaskSwissPoolIndexSchema.optional(),
        tasks: z
          .array(TaskChunkSchema)
          .meta({
            title: "TaskChunkArray",
            recursive: true,
          })
          .describe("The tasks executed as part of the function execution."),
      })
    )
    .describe("A chunk of a function execution task.");

export namespace FunctionExecutionTaskChunk {
  export function merged(
    a: FunctionExecutionTaskChunk,
    b: FunctionExecutionTaskChunk
  ): [FunctionExecutionTaskChunk, boolean] {
    const index = a.index;
    const task_index = a.task_index;
    const task_path = a.task_path;
    const swiss_round = a.swiss_round;
    const swiss_pool_index = a.swiss_pool_index;
    const [base, baseChanged] = FunctionExecutionChunk.merged(a, b);
    if (baseChanged) {
      return [
        {
          index,
          task_index,
          task_path,
          swiss_round,
          swiss_pool_index,
          ...base,
        },
        true,
      ];
    } else {
      return [a, false];
    }
  }
}
