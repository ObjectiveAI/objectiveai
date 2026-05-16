package tests

import (
	"context"
	"fmt"
	"path/filepath"
	"testing"

	. "github.com/ObjectiveAI/objectiveai/objectiveai-sdk-go"
)

func TestVectorCompletionsHTTP(t *testing.T) {
	c := getTestClient(t)
	snapshotsDir := filepath.Join(assetsDir(), "vector", "completions", "client_tests")

	responses25 := make([]AgentCompletionsMessageRichContent, 25)
	for i := range responses25 {
		responses25[i] = AgentCompletionsMessageRichContent{Text: ptr(AgentCompletionsMessageRichContentText(fmt.Sprintf("Response %d", i)))}
	}

	type tc struct {
		snapshot string
		params   VectorCompletionsRequestVectorCompletionCreateParams
	}

	cases := []tc{
		{
			snapshot: "single_agent_2_responses_instruction_seed_42",
			params: VectorCompletionsRequestVectorCompletionCreateParams{
				Messages: []AgentCompletionsMessageMessage{
					{User: &AgentCompletionsMessageMessageUser{
						AgentCompletionsMessageUserMessage: AgentCompletionsMessageUserMessage{Content: AgentCompletionsMessageRichContent{Text: ptr(AgentCompletionsMessageRichContentText("Which is better?"))}},
						Role: "user",
					}},
				},
				Swarm: SwarmInlineSwarmBaseOrRemoteCommitOptional{SwarmBase: &SwarmInlineSwarmBase{Agents: []AgentInlineAgentBaseWithFallbacksOrRemoteWithCount{
					{AgentInlineAgentBaseWithFallbacksOrRemote: AgentInlineAgentBaseWithFallbacksOrRemote{AgentBase: &AgentInlineAgentBaseWithFallbacks{AgentInlineAgentBase: AgentInlineAgentBase{Mock: &AgentMockAgentBase{
						Upstream:   AgentMockUpstream{Mock: "mock"},
						OutputMode: AgentMockOutputMode{Instruction: ptr(AgentMockOutputModeInstruction("instruction"))},
					}}}}, Count: 1},
				}}},
				Responses: []AgentCompletionsMessageRichContent{
					{Text: ptr(AgentCompletionsMessageRichContentText("Response A"))},
					{Text: ptr(AgentCompletionsMessageRichContentText("Response B"))},
				},
				Seed: ptr[int64](42),
			},
		},
		{
			snapshot: "many_responses_deep_prefix_tree_seed_42",
			params: VectorCompletionsRequestVectorCompletionCreateParams{
				Messages: []AgentCompletionsMessageMessage{
					{User: &AgentCompletionsMessageMessageUser{
						AgentCompletionsMessageUserMessage: AgentCompletionsMessageUserMessage{Content: AgentCompletionsMessageRichContent{Text: ptr(AgentCompletionsMessageRichContentText("Pick the best"))}},
						Role: "user",
					}},
				},
				Swarm: SwarmInlineSwarmBaseOrRemoteCommitOptional{SwarmBase: &SwarmInlineSwarmBase{Agents: []AgentInlineAgentBaseWithFallbacksOrRemoteWithCount{
					{AgentInlineAgentBaseWithFallbacksOrRemote: AgentInlineAgentBaseWithFallbacksOrRemote{AgentBase: &AgentInlineAgentBaseWithFallbacks{AgentInlineAgentBase: AgentInlineAgentBase{Mock: &AgentMockAgentBase{
						Upstream:   AgentMockUpstream{Mock: "mock"},
						OutputMode: AgentMockOutputMode{Instruction: ptr(AgentMockOutputModeInstruction("instruction"))},
					}}}}, Count: 1},
				}}},
				Responses: responses25,
				Seed:      ptr[int64](42),
			},
		},
		{
			snapshot: "mixed_output_modes_seed_88",
			params: VectorCompletionsRequestVectorCompletionCreateParams{
				Messages: []AgentCompletionsMessageMessage{
					{User: &AgentCompletionsMessageMessageUser{
						AgentCompletionsMessageUserMessage: AgentCompletionsMessageUserMessage{Content: AgentCompletionsMessageRichContent{Text: ptr(AgentCompletionsMessageRichContentText("Compare these vacation destinations"))}},
						Role: "user",
					}},
				},
				Swarm: SwarmInlineSwarmBaseOrRemoteCommitOptional{SwarmBase: &SwarmInlineSwarmBase{
					Agents: []AgentInlineAgentBaseWithFallbacksOrRemoteWithCount{
						{AgentInlineAgentBaseWithFallbacksOrRemote: AgentInlineAgentBaseWithFallbacksOrRemote{AgentBase: &AgentInlineAgentBaseWithFallbacks{AgentInlineAgentBase: AgentInlineAgentBase{Mock: &AgentMockAgentBase{
							Upstream:   AgentMockUpstream{Mock: "mock"},
							OutputMode: AgentMockOutputMode{Instruction: ptr(AgentMockOutputModeInstruction("instruction"))},
						}}}}, Count: 1},
						{AgentInlineAgentBaseWithFallbacksOrRemote: AgentInlineAgentBaseWithFallbacksOrRemote{AgentBase: &AgentInlineAgentBaseWithFallbacks{AgentInlineAgentBase: AgentInlineAgentBase{Mock: &AgentMockAgentBase{
							Upstream:   AgentMockUpstream{Mock: "mock"},
							OutputMode: AgentMockOutputMode{JsonSchema: ptr(AgentMockOutputModeJsonSchema("json_schema"))},
						}}}}, Count: 1},
						{AgentInlineAgentBaseWithFallbacksOrRemote: AgentInlineAgentBaseWithFallbacksOrRemote{AgentBase: &AgentInlineAgentBaseWithFallbacks{AgentInlineAgentBase: AgentInlineAgentBase{Mock: &AgentMockAgentBase{
							Upstream:   AgentMockUpstream{Mock: "mock"},
							OutputMode: AgentMockOutputMode{ToolCall: ptr(AgentMockOutputModeToolCall("tool_call"))},
						}}}}, Count: 1},
					},
					Weights: &Weights{Weights: ptr(WeightsWeights([]float64{0.4, 0.3, 0.3}))},
				}},
				Responses: []AgentCompletionsMessageRichContent{
					{Text: ptr(AgentCompletionsMessageRichContentText("Kyoto, Japan"))},
					{Text: ptr(AgentCompletionsMessageRichContentText("Reykjavik, Iceland"))},
					{Text: ptr(AgentCompletionsMessageRichContentText("Patagonia, Argentina"))},
				},
				Seed: ptr[int64](88),
			},
		},
	}

	for _, tc := range cases {
		t.Run(tc.snapshot+"/unary", func(t *testing.T) {
			expected := loadSnapshot(t, snapshotsDir, tc.snapshot)
			result, err := VectorCompletionsCreateVectorCompletionUnary(context.Background(), c, tc.params)
			if err != nil {
				t.Fatalf("unary: %v", err)
			}
			normalized, err := NormalizeVectorCompletionForTests(*result)
			if err != nil {
				t.Fatalf("normalize: %v", err)
			}
			assertRoundedMapEqual(t, tc.snapshot, toMapJSON(t, normalized), expected)
		})

		t.Run(tc.snapshot+"/streaming", func(t *testing.T) {
			expected := loadSnapshot(t, snapshotsDir, tc.snapshot)
			stream, err := VectorCompletionsCreateVectorCompletionStreaming(context.Background(), c, tc.params)
			if err != nil {
				t.Fatalf("streaming: %v", err)
			}
			result := accumulateStream(t, stream,
				func(acc, chunk *VectorCompletionsResponseStreamingVectorCompletionChunk) { acc.Push(chunk) },
				VectorCompletionChunkToUnary,
			)
			normalized, err := NormalizeVectorCompletionForTests(*result)
			if err != nil {
				t.Fatalf("normalize: %v", err)
			}
			assertRoundedMapEqual(t, tc.snapshot, toMapJSON(t, normalized), expected)
		})
	}
}
