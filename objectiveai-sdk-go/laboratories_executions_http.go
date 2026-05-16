package objectiveai

import "context"

func LaboratoriesExecutionsCreateLaboratoryExecutionUnary(ctx context.Context, c *Client, params LaboratoriesExecutionsRequestLaboratoryExecutionCreateParams) (*LaboratoriesExecutionsResponseUnaryLaboratoryExecution, error) {
	params.Stream = nil
	return PostUnary[LaboratoriesExecutionsResponseUnaryLaboratoryExecution](ctx, c, "laboratories/executions", params)
}

func LaboratoriesExecutionsCreateLaboratoryExecutionStreaming(ctx context.Context, c *Client, params LaboratoriesExecutionsRequestLaboratoryExecutionCreateParams) (*Stream[LaboratoriesExecutionsResponseStreamingLaboratoryExecutionChunk], error) {
	params.Stream = ptrBool(true)
	return PostStreaming[LaboratoriesExecutionsResponseStreamingLaboratoryExecutionChunk](ctx, c, "laboratories/executions", params)
}
