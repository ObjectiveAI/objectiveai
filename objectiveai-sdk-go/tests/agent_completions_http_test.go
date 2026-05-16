package tests

import (
	"context"
	"path/filepath"
	"testing"

	. "github.com/ObjectiveAI/objectiveai/objectiveai-sdk-go"
)

func ptr[T any](v T) *T { return &v }


func TestAgentCompletionsHTTP(t *testing.T) {
	c := getTestClient(t)
	snapshotsDir := filepath.Join(assetsDir(), "agent", "completions", "client_tests")

	type tc struct {
		snapshot string
		params   AgentCompletionsRequestAgentCompletionCreateParams
	}

	cases := []tc{
		{
			snapshot: "test_basic_mock_agent_seed_42",
			params: AgentCompletionsRequestAgentCompletionCreateParams{
				Messages: []AgentCompletionsMessageMessage{},
				Agent: AgentInlineAgentBaseWithFallbacksOrRemoteCommitOptional{AgentBase: &AgentInlineAgentBaseWithFallbacks{AgentInlineAgentBase: AgentInlineAgentBase{Mock: &AgentMockAgentBase{
					Upstream:   AgentMockUpstream{Mock: "mock"},
					OutputMode: AgentMockOutputMode{Instruction: ptr(AgentMockOutputModeInstruction("instruction"))},
				}}}},
				Seed: ptr[int64](42),
			},
		},
		{
			snapshot: "test_with_developer_and_user_messages",
			params: AgentCompletionsRequestAgentCompletionCreateParams{
				Messages: []AgentCompletionsMessageMessage{
					{Developer: &AgentCompletionsMessageMessageDeveloper{
						AgentCompletionsMessageDeveloperMessage: AgentCompletionsMessageDeveloperMessage{Content: AgentCompletionsMessageSimpleContent{Text: ptr(AgentCompletionsMessageSimpleContentText("You are a helpful assistant."))}},
						Role: "developer",
					}},
					{User: &AgentCompletionsMessageMessageUser{
						AgentCompletionsMessageUserMessage: AgentCompletionsMessageUserMessage{Content: AgentCompletionsMessageRichContent{Text: ptr(AgentCompletionsMessageRichContentText("What is 2+2?"))}},
						Role: "user",
					}},
				},
				Agent: AgentInlineAgentBaseWithFallbacksOrRemoteCommitOptional{AgentBase: &AgentInlineAgentBaseWithFallbacks{AgentInlineAgentBase: AgentInlineAgentBase{Mock: &AgentMockAgentBase{
					Upstream:   AgentMockUpstream{Mock: "mock"},
					OutputMode: AgentMockOutputMode{Instruction: ptr(AgentMockOutputModeInstruction("instruction"))},
				}}}},
				Seed: ptr[int64](99),
			},
		},
		{
			snapshot: "test_json_object_response_format",
			params: AgentCompletionsRequestAgentCompletionCreateParams{
				Messages: []AgentCompletionsMessageMessage{},
				Agent: AgentInlineAgentBaseWithFallbacksOrRemoteCommitOptional{AgentBase: &AgentInlineAgentBaseWithFallbacks{AgentInlineAgentBase: AgentInlineAgentBase{Mock: &AgentMockAgentBase{
					Upstream:   AgentMockUpstream{Mock: "mock"},
					OutputMode: AgentMockOutputMode{Instruction: ptr(AgentMockOutputModeInstruction("instruction"))},
				}}}},
				ResponseFormat: &AgentCompletionsRequestResponseFormatParam{Single: &AgentCompletionsRequestResponseFormat{
					JsonObject: &AgentCompletionsRequestResponseFormatJsonObject{Type: "json_object"},
				}},
				Seed: ptr[int64](42),
			},
		},
	}

	for _, tc := range cases {
		t.Run(tc.snapshot+"/unary", func(t *testing.T) {
			expected := loadSnapshot(t, snapshotsDir, tc.snapshot)
			result, err := AgentCompletionsCreateAgentCompletionUnary(context.Background(), c, tc.params)
			if err != nil {
				t.Fatalf("unary: %v", err)
			}
			normalized, err := NormalizeAgentCompletionForTests(*result)
			if err != nil {
				t.Fatalf("normalize: %v", err)
			}
			assertRoundedMapEqual(t, tc.snapshot, toMapJSON(t, normalized), expected)
		})

		t.Run(tc.snapshot+"/streaming", func(t *testing.T) {
			expected := loadSnapshot(t, snapshotsDir, tc.snapshot)
			stream, err := AgentCompletionsCreateAgentCompletionStreaming(context.Background(), c, tc.params)
			if err != nil {
				t.Fatalf("streaming: %v", err)
			}
			result := accumulateStream(t, stream,
				func(acc, chunk *AgentCompletionsResponseStreamingAgentCompletionChunk) { acc.Push(chunk) },
				AgentCompletionChunkToUnary,
			)
			normalized, err := NormalizeAgentCompletionForTests(*result)
			if err != nil {
				t.Fatalf("normalize: %v", err)
			}
			assertRoundedMapEqual(t, tc.snapshot, toMapJSON(t, normalized), expected)
		})
	}
}
