// Strict roundtrip test harness for Go JSON Schema validation.
//
// THIS FILE MUST NEVER BE MODIFIED.
//
// This harness is purposefully strict. It loads the original JSON schemas from
// objectiveai-json-schema/ exactly as they are on disk -- no normalization, no
// massaging, no xfail. The original schema is treated as the canonical source
// of truth and is never altered.
//
// The contract is simple: the caller passes a schema title and a map. This
// harness loads the original, serializes both sides using the canonical key
// ordering from the JSON schema builder (objectiveai-json-schema/builder/),
// and compares the serialized strings for exact equality.
//
// Key ordering rules (matching the Rust builder):
//   - Inside "properties": keys are sorted alphabetically.
//   - Outside "properties": keys are sorted by KEYWORD_ORDER, with any
//     unknown keys placed at the end.
//
// If a test fails, the fix belongs in the caller's conversion/normalization
// logic or in the Go code generator -- never in this file.
package tests

import (
	"encoding/json"
	"os"
	"path/filepath"
	"runtime"
	"sort"
	"strings"
	"testing"
)

// Canonical key ordering for JSON Schema keywords.
// Matches KEYWORD_ORDER in objectiveai-json-schema/builder/src/main.rs.
var keywordOrder = []string{
	"title", "description", "type", "enum", "anyOf", "$ref",
	"properties", "additionalProperties", "items",
	"minItems", "maxItems", "minimum", "maximum",
	"pattern", "format", "default",
}

var keywordRank map[string]int

func init() {
	keywordRank = make(map[string]int, len(keywordOrder))
	for i, kw := range keywordOrder {
		keywordRank[kw] = i
	}
}

// ---------------------------------------------------------------------------
// Schema loading
// ---------------------------------------------------------------------------

func schemaDir() string {
	_, filename, _, _ := runtime.Caller(0)
	return filepath.Join(filepath.Dir(filename), "..", "..", "objectiveai-json-schema")
}

var (
	OriginalSchemas map[string]map[string]any
	AllTitlesSorted  []string
)

func init() {
	dir := schemaDir()
	entries, err := os.ReadDir(dir)
	if err != nil {
		panic("reading schema dir " + dir + ": " + err.Error())
	}
	OriginalSchemas = make(map[string]map[string]any)
	for _, entry := range entries {
		if entry.IsDir() || !strings.HasSuffix(entry.Name(), ".json") {
			continue
		}
		data, err := os.ReadFile(filepath.Join(dir, entry.Name()))
		if err != nil {
			panic("reading " + entry.Name() + ": " + err.Error())
		}
		dec := json.NewDecoder(strings.NewReader(string(data)))
		dec.UseNumber()
		var schema map[string]any
		if err := dec.Decode(&schema); err != nil {
			panic("parsing " + entry.Name() + ": " + err.Error())
		}
		if title, ok := schema["title"].(string); ok {
			OriginalSchemas[title] = schema
		}
	}
	AllTitlesSorted = make([]string, 0, len(OriginalSchemas))
	for title := range OriginalSchemas {
		AllTitlesSorted = append(AllTitlesSorted, title)
	}
	sort.Strings(AllTitlesSorted)
}

// ---------------------------------------------------------------------------
// Serialization + comparison
// ---------------------------------------------------------------------------

type orderedMap struct {
	keys   []string
	values map[string]any
}

func (o *orderedMap) MarshalJSON() ([]byte, error) {
	var buf strings.Builder
	buf.WriteByte('{')
	for i, k := range o.keys {
		if i > 0 {
			buf.WriteByte(',')
		}
		keyBytes, _ := json.Marshal(k)
		buf.Write(keyBytes)
		buf.WriteByte(':')
		valBytes, _ := json.Marshal(o.values[k])
		buf.Write(valBytes)
	}
	buf.WriteByte('}')
	return []byte(buf.String()), nil
}

func orderKeys(value any, insideProperties bool) any {
	switch v := value.(type) {
	case map[string]any:
		recursed := make(map[string]any, len(v))
		for k, val := range v {
			recursed[k] = orderKeys(val, k == "properties")
		}
		keys := make([]string, 0, len(recursed))
		for k := range recursed {
			keys = append(keys, k)
		}
		if insideProperties {
			sort.Strings(keys)
		} else {
			unknownRank := len(keywordOrder)
			sort.SliceStable(keys, func(i, j int) bool {
				ri, oki := keywordRank[keys[i]]
				if !oki {
					ri = unknownRank
				}
				rj, okj := keywordRank[keys[j]]
				if !okj {
					rj = unknownRank
				}
				return ri < rj
			})
		}
		return &orderedMap{keys: keys, values: recursed}
	case []any:
		result := make([]any, len(v))
		for i, item := range v {
			result[i] = orderKeys(item, false)
		}
		return result
	default:
		return value
	}
}

func serialize(schema map[string]any) string {
	ordered := orderKeys(schema, false)
	data, _ := json.MarshalIndent(ordered, "", "  ")
	return string(data)
}

// AssertSchemaMatches asserts that a converted schema exactly matches the
// original on disk. Both are serialized using canonical key ordering before
// comparison.
func AssertSchemaMatches(t *testing.T, title string, converted map[string]any) {
	t.Helper()
	original, ok := OriginalSchemas[title]
	if !ok {
		t.Fatalf("title %q not found in original schemas", title)
	}
	expected := serialize(original)
	actual := serialize(converted)
	if actual != expected {
		t.Errorf("Schema mismatch for %q:\n\n--- Expected ---\n%s\n\n--- Got ---\n%s",
			title, expected, actual)
	}
}

// SourceDir returns the path to the objectiveai-go/ source directory.
func SourceDir() string {
	_, filename, _, _ := runtime.Caller(0)
	return filepath.Join(filepath.Dir(filename), "..")
}

// RepoRoot returns the path to the repository root.
func RepoRoot() string {
	_, filename, _, _ := runtime.Caller(0)
	return filepath.Join(filepath.Dir(filename), "..", "..")
}
