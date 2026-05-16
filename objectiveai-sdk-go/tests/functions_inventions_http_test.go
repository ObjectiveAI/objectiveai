package tests

import (
	"context"
	"path/filepath"
	"testing"

	. "github.com/ObjectiveAI/objectiveai/objectiveai-sdk-go"
	orderedmap "github.com/wk8/go-ordered-map/v2"
)

type SP = orderedmap.Pair[string, FunctionsExpressionInputSchema]

func schemaPairs(pairs ...SP) OrderedMap[string, FunctionsExpressionInputSchema] {
	return NewOrderedMap[string, FunctionsExpressionInputSchema](pairs...)
}

func TestFunctionsInventionsHTTP(t *testing.T) {
	c := getTestClient(t)
	snapshotsDir := filepath.Join(assetsDir(), "functions", "inventions", "client_tests")

	type tc struct {
		snapshot string
		params   FunctionsInventionsRequestFunctionInventionCreateParams
	}

	cases := []tc{
		{
			snapshot: "scalar_leaf_s42_0",
			params: FunctionsInventionsRequestFunctionInventionCreateParams{
				Agent: AgentInlineAgentBaseWithFallbacksOrRemoteCommitOptional{AgentBase: &AgentInlineAgentBaseWithFallbacks{AgentInlineAgentBase: AgentInlineAgentBase{Mock: &AgentMockAgentBase{
					Upstream:   AgentMockUpstream{Mock: "mock"},
					OutputMode: AgentMockOutputMode{Instruction: ptr(AgentMockOutputModeInstruction("instruction"))},
					Mode:       &AgentMockMode{Invention: ptr("invention")},
				}}}},
				State: FunctionsInventionsStateParamsStateOrRemoteCommitOptional{ParamsState: &FunctionsInventionsStateParamsState{
					AlphaScalarLeaf: &FunctionsInventionsStateParamsStateAlphaScalarLeaf{
						FunctionsInventionsStateAlphaScalarLeafState: FunctionsInventionsStateAlphaScalarLeafState{
							Depth: 0, MinBranchWidth: 3, MaxBranchWidth: 5, MinLeafWidth: 3, MaxLeafWidth: 5,
							Name: "sl-default", Spec: "Test function spec for mock invention.",
						},
						Type: "alpha.scalar.leaf.function",
					},
				}},
				Prompt: FunctionsInventionsPromptsInlinePromptOrRemoteCommitOptional{Remote: &RemotePathCommitOptional{Mock: &RemotePathCommitOptionalMock{Name: "default", Remote: "mock"}}},
				Seed:           ptr[int64](42),
				MaxStepRetries: ptr[uint32](1),
			},
		},
		{
			snapshot: "vector_branch_s2025_0",
			params: FunctionsInventionsRequestFunctionInventionCreateParams{
				Agent: AgentInlineAgentBaseWithFallbacksOrRemoteCommitOptional{AgentBase: &AgentInlineAgentBaseWithFallbacks{AgentInlineAgentBase: AgentInlineAgentBase{Mock: &AgentMockAgentBase{
					Upstream:   AgentMockUpstream{Mock: "mock"},
					OutputMode: AgentMockOutputMode{Instruction: ptr(AgentMockOutputModeInstruction("instruction"))},
					Mode:       &AgentMockMode{Invention: ptr("invention")},
				}}}},
				State: FunctionsInventionsStateParamsStateOrRemoteCommitOptional{ParamsState: &FunctionsInventionsStateParamsState{
					AlphaVectorBranch: &FunctionsInventionsStateParamsStateAlphaVectorBranch{
						FunctionsInventionsStateAlphaVectorBranchState: FunctionsInventionsStateAlphaVectorBranchState{
							Depth: 3, MinBranchWidth: 2, MaxBranchWidth: 4, MinLeafWidth: 2, MaxLeafWidth: 4,
							Name: "vb-deep", Spec: "Test function spec for mock invention.",
						},
						Type: "alpha.vector.branch.function",
					},
				}},
				Prompt: FunctionsInventionsPromptsInlinePromptOrRemoteCommitOptional{Remote: &RemotePathCommitOptional{Mock: &RemotePathCommitOptionalMock{Name: "default", Remote: "mock"}}},
				Seed:           ptr[int64](2025),
				MaxStepRetries: ptr[uint32](1),
			},
		},
		{
			snapshot: "scalar_leaf_schema_kitchen_0",
			params: FunctionsInventionsRequestFunctionInventionCreateParams{
				Agent: AgentInlineAgentBaseWithFallbacksOrRemoteCommitOptional{AgentBase: &AgentInlineAgentBaseWithFallbacks{AgentInlineAgentBase: AgentInlineAgentBase{Mock: &AgentMockAgentBase{
					Upstream:   AgentMockUpstream{Mock: "mock"},
					OutputMode: AgentMockOutputMode{Instruction: ptr(AgentMockOutputModeInstruction("instruction"))},
					Mode:       &AgentMockMode{Invention: ptr("invention")},
				}}}},
				State: FunctionsInventionsStateParamsStateOrRemoteCommitOptional{ParamsState: &FunctionsInventionsStateParamsState{
					AlphaScalarLeaf: &FunctionsInventionsStateParamsStateAlphaScalarLeaf{
						FunctionsInventionsStateAlphaScalarLeafState: FunctionsInventionsStateAlphaScalarLeafState{
							Depth: 0, MinBranchWidth: 3, MaxBranchWidth: 5, MinLeafWidth: 3, MaxLeafWidth: 5,
							Name: "sl-kitchen", Spec: "Test function spec for mock invention.",
							InputSchema: &FunctionsExpressionObjectInputSchema{
								Type: FunctionsExpressionObjectInputSchemaType{Object: "object"},
								Properties: schemaPairs(
									SP{Key: "name", Value: FunctionsExpressionInputSchema{String: &FunctionsExpressionStringInputSchema{Type: FunctionsExpressionStringInputSchemaType{String: "string"}}}},
									SP{Key: "age", Value: FunctionsExpressionInputSchema{Integer: &FunctionsExpressionIntegerInputSchema{Type: FunctionsExpressionIntegerInputSchemaType{Integer: "integer"}}}},
									SP{Key: "score", Value: FunctionsExpressionInputSchema{Number: &FunctionsExpressionNumberInputSchema{Type: FunctionsExpressionNumberInputSchemaType{Number: "number"}}}},
									SP{Key: "active", Value: FunctionsExpressionInputSchema{Boolean: &FunctionsExpressionBooleanInputSchema{Type: FunctionsExpressionBooleanInputSchemaType{Boolean: "boolean"}}}},
									SP{Key: "avatar", Value: FunctionsExpressionInputSchema{Image: &FunctionsExpressionImageInputSchema{Type: FunctionsExpressionImageInputSchemaType{Image: "image"}}}},
									SP{Key: "voicemail", Value: FunctionsExpressionInputSchema{Audio: &FunctionsExpressionAudioInputSchema{Type: FunctionsExpressionAudioInputSchemaType{Audio: "audio"}}}},
									SP{Key: "demo", Value: FunctionsExpressionInputSchema{Video: &FunctionsExpressionVideoInputSchema{Type: FunctionsExpressionVideoInputSchemaType{Video: "video"}}}},
									SP{Key: "resume", Value: FunctionsExpressionInputSchema{File: &FunctionsExpressionFileInputSchema{Type: FunctionsExpressionFileInputSchemaType{File: "file"}}}},
									SP{Key: "aliases", Value: FunctionsExpressionInputSchema{Array: &FunctionsExpressionArrayInputSchema{
										Type:     FunctionsExpressionArrayInputSchemaType{Array: "array"},
										Items:    FunctionsExpressionInputSchema{AnyOf: &FunctionsExpressionAnyOfInputSchema{AnyOf: []FunctionsExpressionInputSchema{{String: &FunctionsExpressionStringInputSchema{Type: FunctionsExpressionStringInputSchemaType{String: "string"}}}, {Integer: &FunctionsExpressionIntegerInputSchema{Type: FunctionsExpressionIntegerInputSchemaType{Integer: "integer"}}}}}},
										MinItems: ptr[uint64](1), MaxItems: ptr[uint64](8),
									}}},
									SP{Key: "extra", Value: FunctionsExpressionInputSchema{AnyOf: &FunctionsExpressionAnyOfInputSchema{AnyOf: []FunctionsExpressionInputSchema{
										{String: &FunctionsExpressionStringInputSchema{Type: FunctionsExpressionStringInputSchemaType{String: "string"}}},
										{Array: &FunctionsExpressionArrayInputSchema{
											Type: FunctionsExpressionArrayInputSchemaType{Array: "array"},
											Items: FunctionsExpressionInputSchema{Object: &FunctionsExpressionObjectInputSchema{
												Type: FunctionsExpressionObjectInputSchemaType{Object: "object"},
												Properties: schemaPairs(
													SP{Key: "key", Value: FunctionsExpressionInputSchema{String: &FunctionsExpressionStringInputSchema{Type: FunctionsExpressionStringInputSchemaType{String: "string"}}}},
													SP{Key: "val", Value: FunctionsExpressionInputSchema{AnyOf: &FunctionsExpressionAnyOfInputSchema{AnyOf: []FunctionsExpressionInputSchema{
														{Number: &FunctionsExpressionNumberInputSchema{Type: FunctionsExpressionNumberInputSchemaType{Number: "number"}}},
														{Boolean: &FunctionsExpressionBooleanInputSchema{Type: FunctionsExpressionBooleanInputSchemaType{Boolean: "boolean"}}},
														{Image: &FunctionsExpressionImageInputSchema{Type: FunctionsExpressionImageInputSchemaType{Image: "image"}}},
													}}}},
												),
												Required: &[]string{"key", "val"},
											}},
											MinItems: ptr[uint64](1), MaxItems: ptr[uint64](3),
										}},
									}}}},
								),
								Required: &[]string{"name", "age", "score", "active", "avatar", "voicemail", "demo", "resume", "aliases", "extra"},
							},
						},
						Type: "alpha.scalar.leaf.function",
					},
				}},
				Prompt: FunctionsInventionsPromptsInlinePromptOrRemoteCommitOptional{Remote: &RemotePathCommitOptional{Mock: &RemotePathCommitOptionalMock{Name: "default", Remote: "mock"}}},
				Seed:           ptr[int64](80004),
				MaxStepRetries: ptr[uint32](1),
			},
		},
	}

	for _, tc := range cases {
		t.Run(tc.snapshot+"/unary", func(t *testing.T) {
			expected := loadSnapshot(t, snapshotsDir, tc.snapshot)
			result, err := FunctionsInventionsCreateFunctionInventionUnary(context.Background(), c, tc.params)
			if err != nil {
				t.Fatalf("unary: %v", err)
			}
			normalized, err := NormalizeFunctionInventionForTests(*result)
			if err != nil {
				t.Fatalf("normalize: %v", err)
			}
			assertRoundedMapEqual(t, tc.snapshot, toMapJSON(t, normalized), expected)
		})

		t.Run(tc.snapshot+"/streaming", func(t *testing.T) {
			expected := loadSnapshot(t, snapshotsDir, tc.snapshot)
			stream, err := FunctionsInventionsCreateFunctionInventionStreaming(context.Background(), c, tc.params)
			if err != nil {
				t.Fatalf("streaming: %v", err)
			}
			result := accumulateStream(t, stream,
				func(acc, chunk *FunctionsInventionsResponseStreamingFunctionInventionChunk) { acc.Push(chunk) },
				FunctionInventionChunkToUnary,
			)
			normalized, err := NormalizeFunctionInventionForTests(*result)
			if err != nil {
				t.Fatalf("normalize: %v", err)
			}
			assertRoundedMapEqual(t, tc.snapshot, toMapJSON(t, normalized), expected)
		})
	}
}
