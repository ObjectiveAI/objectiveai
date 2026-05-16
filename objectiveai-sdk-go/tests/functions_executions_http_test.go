package tests

import (
	"context"
	"path/filepath"
	"testing"

	. "github.com/ObjectiveAI/objectiveai/objectiveai-sdk-go"
	orderedmap "github.com/wk8/go-ordered-map/v2"
)

func TestFunctionsExecutionsHTTP(t *testing.T) {
	c := getTestClient(t)
	snapshotsDir := filepath.Join(assetsDir(), "functions", "executions", "client_tests")

	type tc struct {
		snapshot string
		params   FunctionsExecutionsRequestFunctionExecutionCreateParams
	}

	cases := []tc{
		{
			snapshot: "mock_1_scalar_leaf_binary_seed_42",
			params: FunctionsExecutionsRequestFunctionExecutionCreateParams{
				Function: FunctionsFullInlineFunctionOrRemoteCommitOptional{Remote: &RemotePathCommitOptional{Mock: &RemotePathCommitOptionalMock{Remote: "mock", Name: "binary-classifier"}}},
				Profile:  FunctionsInlineProfileOrRemoteCommitOptional{Remote: &RemotePathCommitOptional{Mock: &RemotePathCommitOptionalMock{Remote: "mock", Name: "solo-instruction"}}},
				Input: FunctionsExpressionInputValue{Object: ptr(NewFunctionsExpressionInputValueObject(
					orderedmap.Pair[string, FunctionsExpressionInputValue]{Key: "text", Value: FunctionsExpressionInputValue{String: ptr(FunctionsExpressionInputValueString("Hello world"))}},
				))},
				Seed: ptr[int64](42),
			},
		},
		{
			snapshot: "mock_7_vector_5_criteria_seed_42",
			params: FunctionsExecutionsRequestFunctionExecutionCreateParams{
				Function: FunctionsFullInlineFunctionOrRemoteCommitOptional{Remote: &RemotePathCommitOptional{Mock: &RemotePathCommitOptionalMock{Remote: "mock", Name: "five-criteria-ranker"}}},
				Profile:  FunctionsInlineProfileOrRemoteCommitOptional{Remote: &RemotePathCommitOptional{Mock: &RemotePathCommitOptionalMock{Remote: "mock", Name: "schema-heavy-trio"}}},
				Input: FunctionsExpressionInputValue{Object: ptr(NewFunctionsExpressionInputValueObject(
					orderedmap.Pair[string, FunctionsExpressionInputValue]{Key: "items", Value: FunctionsExpressionInputValue{Array: ptr(FunctionsExpressionInputValueArray{
						{String: ptr(FunctionsExpressionInputValueString("Option A"))},
						{String: ptr(FunctionsExpressionInputValueString("Option B"))},
						{String: ptr(FunctionsExpressionInputValueString("Option C"))},
					})}},
				))},
				Seed: ptr[int64](42),
			},
		},
		{
			snapshot: "mock_20_vector_super_branch_seed_42",
			params: FunctionsExecutionsRequestFunctionExecutionCreateParams{
				Function: FunctionsFullInlineFunctionOrRemoteCommitOptional{Remote: &RemotePathCommitOptional{Mock: &RemotePathCommitOptionalMock{Remote: "mock", Name: "nested-vector-super-branch"}}},
				Profile:  FunctionsInlineProfileOrRemoteCommitOptional{Remote: &RemotePathCommitOptional{Mock: &RemotePathCommitOptionalMock{Remote: "mock", Name: "nested-vector-inline-remote"}}},
				Input: FunctionsExpressionInputValue{Object: ptr(NewFunctionsExpressionInputValueObject(
					orderedmap.Pair[string, FunctionsExpressionInputValue]{Key: "items", Value: FunctionsExpressionInputValue{Array: ptr(FunctionsExpressionInputValueArray{
						{String: ptr(FunctionsExpressionInputValueString("Alpha"))},
						{String: ptr(FunctionsExpressionInputValueString("Beta"))},
						{String: ptr(FunctionsExpressionInputValueString("Gamma"))},
					})}},
				))},
				Seed: ptr[int64](42),
			},
		},
	}

	for _, tc := range cases {
		t.Run(tc.snapshot+"/unary", func(t *testing.T) {
			expected := loadSnapshot(t, snapshotsDir, tc.snapshot)
			result, err := FunctionsExecutionsCreateFunctionExecutionUnary(context.Background(), c, tc.params)
			if err != nil {
				t.Fatalf("unary: %v", err)
			}
			normalized, err := NormalizeFunctionExecutionForTests(*result)
			if err != nil {
				t.Fatalf("normalize: %v", err)
			}
			assertRoundedMapEqual(t, tc.snapshot, toMapJSON(t, normalized), expected)
		})

		t.Run(tc.snapshot+"/streaming", func(t *testing.T) {
			expected := loadSnapshot(t, snapshotsDir, tc.snapshot)
			stream, err := FunctionsExecutionsCreateFunctionExecutionStreaming(context.Background(), c, tc.params)
			if err != nil {
				t.Fatalf("streaming: %v", err)
			}
			result := accumulateStream(t, stream,
				func(acc, chunk *FunctionsExecutionsResponseStreamingFunctionExecutionChunk) { acc.Push(chunk) },
				FunctionExecutionChunkToUnary,
			)
			normalized, err := NormalizeFunctionExecutionForTests(*result)
			if err != nil {
				t.Fatalf("normalize: %v", err)
			}
			assertRoundedMapEqual(t, tc.snapshot, toMapJSON(t, normalized), expected)
		})
	}
}
