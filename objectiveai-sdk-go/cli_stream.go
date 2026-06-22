package objectiveai

import (
	"encoding/json"
	"fmt"
	"io"
)

// CliStream is a typed wrapper over a RawStream's JSONL lines, used by the
// generated per-command execute functions (plugin authors normally receive one
// rather than constructing it). Port of cliStream.ts / cli_stream.py.
//
// Each raw line is decoded as the left-to-right union `CliError | T`: a CliError
// envelope (a line carrying `type:"error"` + `message`) surfaces as a Go error
// value (*CliError) — matching how the SSE Stream[T] surfaces API errors —
// while a response value decodes into T. Before decoding, externally-tagged
// aggregate layers are peeled (see peelSingleKey / extract_leaf). The host's
// synthetic `{"type":"end"}` terminator is consumed and never yielded.
type CliStream[T any] struct {
	raw RawStream
}

// NewCliStream wraps a RawStream so its lines decode into T (or *CliError).
func NewCliStream[T any](raw RawStream) *CliStream[T] {
	return &CliStream[T]{raw: raw}
}

// Next returns the next response, or io.EOF at end of stream. A CliError line is
// returned as the error (a *CliError, type-assertable by the caller).
func (s *CliStream[T]) Next() (*T, error) {
	for {
		line, err := s.raw.Next()
		if err != nil {
			return nil, err // io.EOF and transport errors propagate
		}
		if isEndMarker(line) {
			// The underlying stream terminates itself after the end marker;
			// skipping here just keeps it out of the typed stream.
			continue
		}
		result, cliErr, perr := parseCliLine[T](line)
		if perr != nil {
			return nil, perr
		}
		if cliErr != nil {
			return nil, cliErr
		}
		return result, nil
	}
}

// ToList collects every remaining item, stopping at the first CliError (returned
// as the error) or transport error. io.EOF is the clean terminator (nil error).
func (s *CliStream[T]) ToList() ([]T, error) {
	var items []T
	for {
		item, err := s.Next()
		if err == io.EOF {
			return items, nil
		}
		if err != nil {
			return items, err
		}
		items = append(items, *item)
	}
}

// First returns the first item and discards the rest. (*T, nil) on a value,
// (nil, *CliError) on an error envelope, (nil, nil) when the stream ended
// without yielding (a unary command that printed nothing before the end
// marker), or (nil, err) on a transport error.
func (s *CliStream[T]) First() (*T, error) {
	defer s.raw.Close()
	item, err := s.Next()
	if err == io.EOF {
		return nil, nil
	}
	return item, err
}

// Close releases the underlying stream.
func (s *CliStream[T]) Close() error {
	return s.raw.Close()
}

// parseCliLine decodes one line as the union `CliError | T`, peeling
// externally-tagged single-key layers until a variant accepts — the Go mirror
// of `_validate_unwrapping`. CliError is tried first (left-to-right), so an
// error envelope short-circuits. On total failure the error names the original
// (un-peeled) wire shape.
func parseCliLine[T any](line json.RawMessage) (*T, *CliError, error) {
	current := line
	for {
		// (left) CliError: only a line with `type:"error"` + `message` parses.
		var cliErr CliError
		if json.Unmarshal(current, &cliErr) == nil && cliErr.Validate() == nil {
			return nil, &cliErr, nil
		}
		// (right) the response type T.
		var value T
		if json.Unmarshal(current, &value) == nil {
			if v, ok := any(&value).(interface{ Validate() error }); !ok || v.Validate() == nil {
				return &value, nil, nil
			}
		}
		// Neither matched: peel one aggregate layer and retry.
		if next, ok := peelSingleKey(current); ok {
			current = next
			continue
		}
		return nil, nil, fmt.Errorf(
			"objectiveai: cli line matched neither CliError nor %T: %s",
			value, string(line),
		)
	}
}
