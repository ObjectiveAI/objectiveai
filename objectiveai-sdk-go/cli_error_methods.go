package objectiveai

import "encoding/json"

// Error makes *CliError satisfy the error interface so the typed CliStream can
// surface a CLI error envelope as a Go error (consistent with how the SSE
// Stream[T] surfaces API errors). A plain-string message is used verbatim;
// a structured message is JSON-encoded.
func (e *CliError) Error() string {
	if s, ok := e.Message.Value.(string); ok {
		return "objectiveai cli: " + s
	}
	if b, err := json.Marshal(e.Message); err == nil {
		return "objectiveai cli: " + string(b)
	}
	return "objectiveai cli: error"
}
