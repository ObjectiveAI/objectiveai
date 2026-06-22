package objectiveai

import (
	"context"
	"encoding/json"
)

// CommandExecutor drives the ObjectiveAI CLI. There are two implementations,
// mirroring the JS/Python SDKs:
//
//   - BinaryCommandExecutor spawns `objectiveai --request <json>` and streams
//     its JSONL stdout.
//   - PluginCommandExecutor (used INSIDE a plugin process) asks the host to run
//     a command over the plugin's own stdin/stdout via the NDJSON protocol.
//
// `request` is the typed leaf request (e.g. CliCommandAgentsTagsApplyRequest);
// it is serialized to JSON and handed to the CLI verbatim. The request is
// self-describing via its single-variant `PathType` field, so there is no
// Request -> argv lowering (it never exists in the SDKs). The generated per-leaf
// execute functions build the request and decode the typed response.
type CommandExecutor interface {
	Execute(ctx context.Context, request any) (RawStream, error)
}

// RawStream is a pull-based stream of raw JSONL values produced by a
// CommandExecutor — one json.RawMessage per line, in arrival order. Next
// returns io.EOF when the stream is exhausted. Mirrors the existing SSE
// Stream[T] idiom (Next/Close). The typed CliStream[T] wraps it to discriminate
// errors vs responses; plugin authors normally never touch a RawStream
// directly.
type RawStream interface {
	// Next returns the next raw line, or io.EOF at end of stream.
	Next() (json.RawMessage, error)
	// Close releases the stream's resources (kills the child / unregisters the
	// plugin demux channel). Safe to call more than once.
	Close() error
}

// isEndMarker reports whether a line is the host's synthetic `{"type":"end"}`
// stream terminator, which is consumed (ends iteration) and never yielded.
func isEndMarker(raw json.RawMessage) bool {
	var probe struct {
		Type string `json:"type"`
	}
	return json.Unmarshal(raw, &probe) == nil && probe.Type == "end"
}

// peelSingleKey unwraps one externally-tagged aggregate layer: if `raw` is a
// JSON object with EXACTLY one key, it returns that key's value. Used by
// parseCliLine to peel `{"Agents":{"Tags":{"Apply":{…}}}}` down to the leaf —
// the Go mirror of `extract_leaf` in objectiveai-cli/src/executor.rs.
func peelSingleKey(raw json.RawMessage) (json.RawMessage, bool) {
	var obj map[string]json.RawMessage
	if err := json.Unmarshal(raw, &obj); err != nil || len(obj) != 1 {
		return nil, false
	}
	for _, v := range obj {
		return v, true
	}
	return nil, false
}
