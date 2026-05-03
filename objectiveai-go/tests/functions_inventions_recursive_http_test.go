package tests

import (
	"context"
	"path/filepath"
	"testing"

	. "github.com/ObjectiveAI/objectiveai/objectiveai-go"
)

func TestFunctionsInventionsRecursiveHTTP(t *testing.T) {
	c := getTestClient(t)
	snapshotsDir := filepath.Join(assetsDir(), "functions", "inventions", "recursive_client_tests")

	type tc struct {
		snapshot string
		params   FunctionsInventionsRecursiveRequestFunctionInventionRecursiveCreateParams
	}

	sentimentSchema := &FunctionsExpressionObjectInputSchema{
		Type: FunctionsExpressionObjectInputSchemaType{Object: "object"},
		Properties: schemaPairs(
			SP{Key: "sentiment", Value: FunctionsExpressionInputSchema{String: &FunctionsExpressionStringInputSchema{Type: FunctionsExpressionStringInputSchemaType{String: "string"}, Enum: &[]string{"positive", "negative"}}}},
		),
		Required: &[]string{"sentiment"},
	}

	cases := []tc{
		{
			snapshot: "valid_schema_valid_tasks_scalar_leaf",
			params: FunctionsInventionsRecursiveRequestFunctionInventionRecursiveCreateParams{
				Remote: Remote{Mock: ptr(RemoteMock("mock"))},
				Agent: AgentInlineAgentBaseWithFallbacksOrRemoteCommitOptional{AgentBase: &AgentInlineAgentBaseWithFallbacks{AgentInlineAgentBase: AgentInlineAgentBase{Mock: &AgentMockAgentBase{
					Upstream:   AgentMockUpstream{Mock: "mock"},
					OutputMode: AgentMockOutputMode{Instruction: ptr(AgentMockOutputModeInstruction("instruction"))},
					Mode:       &AgentMockMode{Invention: ptr("invention")},
				}}}},
				State: FunctionsInventionsStateParamsStateOrRemoteCommitOptional{ParamsState: &FunctionsInventionsStateParamsState{
					AlphaScalarLeaf: &FunctionsInventionsStateParamsStateAlphaScalarLeaf{
						FunctionsInventionsStateAlphaScalarLeafState: FunctionsInventionsStateAlphaScalarLeafState{
							Depth: 0, MinBranchWidth: 1, MaxBranchWidth: 1, MinLeafWidth: 2, MaxLeafWidth: 4,
							Name: "inv-good-sl", Spec: "Test function spec for mock invention.",
							InputSchema: sentimentSchema,
							EssayTasks:  ptr("Good tasks incoming."),
							Tasks: &[]FunctionsAlphaScalarLeafTaskExpression{
								{FunctionsAlphaScalarVectorCompletionTaskExpression: FunctionsAlphaScalarVectorCompletionTaskExpression{
									Messages:  FunctionsExpressionExpression{Starlark: &FunctionsExpressionExpressionStarlark{Starlark: `[{"role": "user", "content": [{"type": "text", "text": str(input)}]}]`}},
									Responses: []AgentCompletionsMessageRichContent{{Parts: ptr(AgentCompletionsMessageRichContentParts{{Text: &AgentCompletionsMessageRichContentPartText{Type: "text", Text: "yes"}}})}, {Parts: ptr(AgentCompletionsMessageRichContentParts{{Text: &AgentCompletionsMessageRichContentPartText{Type: "text", Text: "no"}}})}},
								}, Type: "vector.completion"},
								{FunctionsAlphaScalarVectorCompletionTaskExpression: FunctionsAlphaScalarVectorCompletionTaskExpression{
									Messages:  FunctionsExpressionExpression{Starlark: &FunctionsExpressionExpressionStarlark{Starlark: `[{"role": "user", "content": [{"type": "text", "text": str(input)}]}]`}},
									Responses: []AgentCompletionsMessageRichContent{{Parts: ptr(AgentCompletionsMessageRichContentParts{{Text: &AgentCompletionsMessageRichContentPartText{Type: "text", Text: "yes"}}})}, {Parts: ptr(AgentCompletionsMessageRichContentParts{{Text: &AgentCompletionsMessageRichContentPartText{Type: "text", Text: "no"}}})}},
								}, Type: "vector.completion"},
							},
							TasksLength: ptr[uint64](2),
							Description: ptr("A valid scalar function."),
						},
						Type: "alpha.scalar.leaf.function",
					},
				}},
				Prompt: FunctionsInventionsPromptsInlinePromptOrRemoteCommitOptional{Remote: &RemotePathCommitOptional{Mock: &RemotePathCommitOptionalMock{Name: "default", Remote: "mock"}}},
			Seed:           ptr[int64](5300),
				MaxStepRetries: ptr[uint32](1),
			},
		},
		{
			snapshot: "valid_vector_schema_valid_tasks",
			params: FunctionsInventionsRecursiveRequestFunctionInventionRecursiveCreateParams{
				Remote: Remote{Mock: ptr(RemoteMock("mock"))},
				Agent: AgentInlineAgentBaseWithFallbacksOrRemoteCommitOptional{AgentBase: &AgentInlineAgentBaseWithFallbacks{AgentInlineAgentBase: AgentInlineAgentBase{Mock: &AgentMockAgentBase{
					Upstream:   AgentMockUpstream{Mock: "mock"},
					OutputMode: AgentMockOutputMode{Instruction: ptr(AgentMockOutputModeInstruction("instruction"))},
					Mode:       &AgentMockMode{Invention: ptr("invention")},
				}}}},
				State: FunctionsInventionsStateParamsStateOrRemoteCommitOptional{ParamsState: &FunctionsInventionsStateParamsState{
					AlphaVectorLeaf: &FunctionsInventionsStateParamsStateAlphaVectorLeaf{
						FunctionsInventionsStateAlphaVectorLeafState: FunctionsInventionsStateAlphaVectorLeafState{
							Depth: 0, MinBranchWidth: 1, MaxBranchWidth: 1, MinLeafWidth: 2, MaxLeafWidth: 4,
							Name: "inv-good-vl", Spec: "Test function spec for mock invention.",
							Essay: ptr("Ranking things."),
							InputSchema: &FunctionsAlphaVectorExpressionVectorFunctionInputSchema{
								Items: FunctionsExpressionInputSchema{String: &FunctionsExpressionStringInputSchema{Type: FunctionsExpressionStringInputSchemaType{String: "string"}, Enum: &[]string{"apple", "banana"}}},
							},
							Tasks: &[]FunctionsAlphaVectorLeafTaskExpression{
								{FunctionsAlphaVectorVectorCompletionTaskExpression: FunctionsAlphaVectorVectorCompletionTaskExpression{
									Messages:  FunctionsExpressionExpression{Starlark: &FunctionsExpressionExpressionStarlark{Starlark: `[{"role": "user", "content": [{"type": "text", "text": "rank these"}]}]`}},
									Responses: FunctionsExpressionExpression{Starlark: &FunctionsExpressionExpressionStarlark{Starlark: `[[{"type": "text", "text": str(item)}] for item in input['items']]`}},
								}, Type: "vector.completion"},
								{FunctionsAlphaVectorVectorCompletionTaskExpression: FunctionsAlphaVectorVectorCompletionTaskExpression{
									Messages:  FunctionsExpressionExpression{Starlark: &FunctionsExpressionExpressionStarlark{Starlark: `[{"role": "user", "content": [{"type": "text", "text": "rank these"}]}]`}},
									Responses: FunctionsExpressionExpression{Starlark: &FunctionsExpressionExpressionStarlark{Starlark: `[[{"type": "text", "text": str(item)}] for item in input['items']]`}},
								}, Type: "vector.completion"},
							},
							TasksLength: ptr[uint64](2),
						},
						Type: "alpha.vector.leaf.function",
					},
				}},
				Prompt: FunctionsInventionsPromptsInlinePromptOrRemoteCommitOptional{Remote: &RemotePathCommitOptional{Mock: &RemotePathCommitOptionalMock{Name: "default", Remote: "mock"}}},
			Seed:           ptr[int64](5400),
				MaxStepRetries: ptr[uint32](1),
			},
		},
		{
			snapshot: "valid_schema_no_tasks_with_essay",
			params: FunctionsInventionsRecursiveRequestFunctionInventionRecursiveCreateParams{
				Remote: Remote{Mock: ptr(RemoteMock("mock"))},
				Agent: AgentInlineAgentBaseWithFallbacksOrRemoteCommitOptional{AgentBase: &AgentInlineAgentBaseWithFallbacks{AgentInlineAgentBase: AgentInlineAgentBase{Mock: &AgentMockAgentBase{
					Upstream:   AgentMockUpstream{Mock: "mock"},
					OutputMode: AgentMockOutputMode{Instruction: ptr(AgentMockOutputModeInstruction("instruction"))},
					Mode:       &AgentMockMode{Invention: ptr("invention")},
				}}}},
				State: FunctionsInventionsStateParamsStateOrRemoteCommitOptional{ParamsState: &FunctionsInventionsStateParamsState{
					AlphaScalarLeaf: &FunctionsInventionsStateParamsStateAlphaScalarLeaf{
						FunctionsInventionsStateAlphaScalarLeafState: FunctionsInventionsStateAlphaScalarLeafState{
							Depth: 0, MinBranchWidth: 1, MaxBranchWidth: 1, MinLeafWidth: 2, MaxLeafWidth: 4,
							Name: "inv-schema-only", Spec: "Test function spec for mock invention.",
							Essay:       ptr("A great essay about things."),
							InputSchema: sentimentSchema,
						},
						Type: "alpha.scalar.leaf.function",
					},
				}},
				Prompt: FunctionsInventionsPromptsInlinePromptOrRemoteCommitOptional{Remote: &RemotePathCommitOptional{Mock: &RemotePathCommitOptionalMock{Name: "default", Remote: "mock"}}},
			Seed:           ptr[int64](5900),
				MaxStepRetries: ptr[uint32](1),
			},
		},
	}

	for _, tc := range cases {
		t.Run(tc.snapshot+"/unary", func(t *testing.T) {
			expected := loadSnapshot(t, snapshotsDir, tc.snapshot)
			result, err := FunctionsInventionsRecursiveCreateFunctionInventionRecursiveUnary(context.Background(), c, tc.params)
			if err != nil {
				t.Fatalf("unary: %v", err)
			}
			normalized, err := NormalizeFunctionInventionRecursiveForTests(*result)
			if err != nil {
				t.Fatalf("normalize: %v", err)
			}
			assertRoundedMapEqual(t, tc.snapshot, toMapJSON(t, normalized), expected)
		})

		t.Run(tc.snapshot+"/streaming", func(t *testing.T) {
			expected := loadSnapshot(t, snapshotsDir, tc.snapshot)
			stream, err := FunctionsInventionsRecursiveCreateFunctionInventionRecursiveStreaming(context.Background(), c, tc.params)
			if err != nil {
				t.Fatalf("streaming: %v", err)
			}
			result := accumulateStream(t, stream,
				func(acc, chunk *FunctionsInventionsRecursiveResponseStreamingFunctionInventionRecursiveChunk) {
					acc.Push(chunk)
				},
				FunctionInventionRecursiveChunkToUnary,
			)
			normalized, err := NormalizeFunctionInventionRecursiveForTests(*result)
			if err != nil {
				t.Fatalf("normalize: %v", err)
			}
			assertRoundedMapEqual(t, tc.snapshot, toMapJSON(t, normalized), expected)
		})
	}
}
