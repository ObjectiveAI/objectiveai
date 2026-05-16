package tests

import (
	"encoding/json"
	"fmt"
	"io"
	"math"
	"os"
	"path/filepath"
	"reflect"
	"testing"

	. "github.com/ObjectiveAI/objectiveai/objectiveai-sdk-go"
)

var testPort = os.Getenv("OBJECTIVEAI_TEST_PORT")

func assetsDir() string {
	return filepath.Join(RepoRoot(), "objectiveai-api", "assets")
}

func getTestClient(t *testing.T) *Client {
	t.Helper()
	if testPort == "" {
		t.Skip("OBJECTIVEAI_TEST_PORT not set")
	}
	return NewClient(func(c *Client) {
		c.Address = fmt.Sprintf("http://127.0.0.1:%s", testPort)
	})
}

func loadSnapshot(t *testing.T, dir, name string) map[string]any {
	t.Helper()
	data, err := os.ReadFile(filepath.Join(dir, name+".json"))
	if err != nil {
		t.Fatalf("load snapshot %s: %v", name, err)
	}
	var m map[string]any
	if err := json.Unmarshal(data, &m); err != nil {
		t.Fatalf("parse snapshot %s: %v", name, err)
	}
	return m
}

// rounded normalizes floats to 8 significant figures for comparison.
func rounded(v any) any {
	switch val := v.(type) {
	case bool:
		return val
	case float64:
		if val == 0 || math.IsInf(val, 0) || math.IsNaN(val) {
			return val
		}
		s12 := fmt.Sprintf("%.12g", val)
		var f12 float64
		fmt.Sscanf(s12, "%g", &f12)
		s8 := fmt.Sprintf("%.8g", f12)
		var f8 float64
		fmt.Sscanf(s8, "%g", &f8)
		return f8
	case []any:
		out := make([]any, len(val))
		for i, item := range val {
			out[i] = rounded(item)
		}
		return out
	case map[string]any:
		out := make(map[string]any, len(val))
		for k, item := range val {
			out[k] = rounded(item)
		}
		return out
	default:
		return val
	}
}

// toMapJSON serializes a value to JSON then deserializes to map[string]any.
func toMapJSON(t *testing.T, v any) map[string]any {
	t.Helper()
	data, err := json.Marshal(v)
	if err != nil {
		t.Fatalf("marshal: %v", err)
	}
	var m map[string]any
	if err := json.Unmarshal(data, &m); err != nil {
		t.Fatalf("unmarshal: %v", err)
	}
	return m
}

func assertRoundedMapEqual(t *testing.T, label string, got, want map[string]any) {
	t.Helper()
	gotR := rounded(got)
	wantR := rounded(want)
	if !reflect.DeepEqual(gotR, wantR) {
		gotJSON, _ := json.Marshal(gotR)
		wantJSON, _ := json.Marshal(wantR)
		t.Errorf("%s mismatch:\n  got:  %s\n  want: %s", label, gotJSON, wantJSON)
	}
}

// accumulateStream reads all chunks from a stream, accumulating via push.
func accumulateStream[Chunk any, Unary any](
	t *testing.T,
	stream *Stream[Chunk],
	push func(*Chunk, *Chunk),
	chunkToUnary func(Chunk) (*Unary, error),
) *Unary {
	t.Helper()
	defer stream.Close()

	var acc *Chunk
	for {
		chunk, err := stream.Next()
		if err == io.EOF {
			break
		}
		if err != nil {
			t.Fatalf("stream.Next: %v", err)
		}
		if v, ok := any(chunk).(interface{ Validate() error }); ok {
			if err := v.Validate(); err != nil {
				t.Fatalf("chunk Validate: %v", err)
			}
		}
		if acc == nil {
			acc = chunk
		} else {
			push(acc, chunk)
		}
	}

	if acc == nil {
		t.Fatal("stream yielded no chunks")
	}

	unary, err := chunkToUnary(*acc)
	if err != nil {
		t.Fatalf("chunkToUnary: %v", err)
	}
	return unary
}
