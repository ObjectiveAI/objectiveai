package objectiveai

import (
	"encoding/json"
	"fmt"
	"math"
	"testing"
)

// rounded normalizes floats to 8 significant figures for comparison.
// Double-rounds through 12 digits first to normalize 1-ULP artifacts.
// Mirrors mergeTestUtil.ts rounded() and push_test_utils.py rounded().
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
	case json.Number:
		f, err := val.Float64()
		if err != nil {
			return val
		}
		return rounded(f)
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

// toMap serializes a value to JSON then deserializes to map[string]any
// for comparison (normalizes types through JSON round-trip).
func toMap(t *testing.T, v any) map[string]any {
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

// deepCopy creates a deep copy of a value via JSON round-trip.
func deepCopy[T any](t *testing.T, v *T) T {
	t.Helper()
	data, err := json.Marshal(v)
	if err != nil {
		t.Fatalf("deepCopy marshal: %v", err)
	}
	var copy T
	if err := json.Unmarshal(data, &copy); err != nil {
		t.Fatalf("deepCopy unmarshal: %v", err)
	}
	return copy
}

// assertRoundedEqual compares two values after rounding floats.
func assertRoundedEqual(t *testing.T, label string, got, want any) {
	t.Helper()
	gotR := rounded(got)
	wantR := rounded(want)
	gotJSON, _ := json.Marshal(gotR)
	wantJSON, _ := json.Marshal(wantR)
	if string(gotJSON) != string(wantJSON) {
		t.Errorf("%s mismatch:\n  got:  %s\n  want: %s", label, gotJSON, wantJSON)
	}
}

