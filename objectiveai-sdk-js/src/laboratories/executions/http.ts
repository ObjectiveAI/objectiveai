import { ObjectiveAI, type RequestOptions } from "../../client";
import { Stream } from "../../stream";
import type { LaboratoriesExecutionsRequestLaboratoryExecutionCreateParams } from "./request/laboratoryExecutionCreateParams";
import type { LaboratoriesExecutionsResponseUnaryLaboratoryExecution } from "./response/unary/laboratoryExecution";
import type { LaboratoriesExecutionsResponseStreamingLaboratoryExecutionChunk } from "./response/streaming/laboratoryExecutionChunk";

export function laboratoriesExecutionsCreateLaboratoryExecution(
  client: ObjectiveAI,
  body: LaboratoriesExecutionsRequestLaboratoryExecutionCreateParams & { stream: true },
  options?: RequestOptions,
): Promise<Stream<LaboratoriesExecutionsResponseStreamingLaboratoryExecutionChunk>>;
export function laboratoriesExecutionsCreateLaboratoryExecution(
  client: ObjectiveAI,
  body: LaboratoriesExecutionsRequestLaboratoryExecutionCreateParams & { stream?: false | null },
  options?: RequestOptions,
): Promise<LaboratoriesExecutionsResponseUnaryLaboratoryExecution>;
export function laboratoriesExecutionsCreateLaboratoryExecution(
  client: ObjectiveAI,
  body: LaboratoriesExecutionsRequestLaboratoryExecutionCreateParams,
  options?: RequestOptions,
): Promise<
  | Stream<LaboratoriesExecutionsResponseStreamingLaboratoryExecutionChunk>
  | LaboratoriesExecutionsResponseUnaryLaboratoryExecution
> {
  if (body.stream) {
    return client.post_streaming<LaboratoriesExecutionsResponseStreamingLaboratoryExecutionChunk>(
      "laboratories/executions",
      body,
      options,
    );
  }
  return client.post_unary<LaboratoriesExecutionsResponseUnaryLaboratoryExecution>(
    "laboratories/executions",
    body,
    options,
  );
}
