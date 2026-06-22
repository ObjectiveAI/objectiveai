// E2E test fixture: a plugin written in Go, run by `plugins run` as the
// compiled `./tags-apply-go[.exe]` binary. It uses the Go SDK's in-process
// PluginCommandExecutor + the generated `agents tags apply` execute fn to mutate
// host state (apply a tag to a mock agent) over the NDJSON command protocol,
// then emits one notification and returns.
//
// Unlike the Node fixture, no explicit exit is needed: a Go process terminates
// when main returns, regardless of the background stdin-reader goroutine the
// executor starts.
package main

import (
	"context"
	"encoding/json"
	"fmt"
	"os"

	objectiveai "github.com/ObjectiveAI/objectiveai/objectiveai-sdk-go"
)

const tag = "go-plugin-applied-tag"

func main() {
	// The mock agent spec, built from a JSON literal so we don't hand-construct
	// the deep AgentSpec type (the execute fn only marshals the request).
	var spec objectiveai.AgentInlineAgentBaseWithFallbacksOrRemoteCommitOptional
	if err := json.Unmarshal([]byte(`{"upstream":"mock","output_mode":"instruction"}`), &spec); err != nil {
		fmt.Fprintln(os.Stderr, "tags-apply-go: build agent spec:", err)
		os.Exit(1)
	}

	req := objectiveai.CliCommandAgentsTagsApplyRequest{
		Name: tag,
		Target: objectiveai.CliCommandAgentsTagsApplyTarget{
			Agent: &objectiveai.CliCommandAgentsTagsApplyTargetAgent{
				By:        "agent",
				AgentSpec: spec,
			},
		},
	}

	executor := objectiveai.PluginExecutorInstance()
	if _, err := objectiveai.AgentsTagsApplyExecute(context.Background(), executor, req); err != nil {
		fmt.Fprintln(os.Stderr, "tags-apply-go: apply tag:", err)
		os.Exit(1)
	}

	line, err := json.Marshal(map[string]any{"type": "notification", "applied": tag})
	if err != nil {
		fmt.Fprintln(os.Stderr, "tags-apply-go: marshal notification:", err)
		os.Exit(1)
	}
	os.Stdout.Write(append(line, '\n'))
}
