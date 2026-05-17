package tests

import (
	"os"
	"path/filepath"
	"regexp"
	"sort"
	"strings"
	"testing"
	"unicode"
)

// TestHTTPCoverage asserts that every Rust http.rs file has a corresponding
// Go file in objectiveai-go/, and that every pub async fn in
// Rust has a corresponding exported Go function.
//
// Naming convention:
//   Rust module path: agent/completions/http.rs
//   Go file:          agent_completions_http.go
//   Rust fn:          create_agent_completion_unary
//   Go fn:            AgentCompletionsCreateAgentCompletionUnary (PascalCase, 1:1 with Rust)
func TestHTTPCoverage(t *testing.T) {
	repoRoot := RepoRoot()
	rustSrc := filepath.Join(repoRoot, "objectiveai-sdk-rs", "src")
	goSrc := SourceDir()

	// Find all Rust http.rs files
	rustHTTPFiles := findHTTPFiles(t, rustSrc)
	if len(rustHTTPFiles) == 0 {
		t.Fatal("Found no http.rs files in objectiveai-sdk-rs/src/")
	}

	// Check each Rust http.rs has a Go counterpart
	var allErrors []string
	for _, entry := range rustHTTPFiles {
		goFileName := entry.modulePath + "_http.go"
		goFilePath := filepath.Join(goSrc, goFileName)

		goBytes, err := os.ReadFile(goFilePath)
		if err != nil {
			allErrors = append(allErrors, entry.modulePath+": missing Go file "+goFileName)
			continue
		}
		goSrc := string(goBytes)

		// Extract Go exported functions
		goFnRe := regexp.MustCompile(`func\s+([A-Z]\w*)\s*[(\[]`)
		goMatches := goFnRe.FindAllStringSubmatch(goSrc, -1)
		goFnSet := make(map[string]bool)
		for _, m := range goMatches {
			goFnSet[m[1]] = true
		}

		// Extract Rust functions (1:1 mapping, no collapsing)
		rustFns := extractRustHTTPFunctions(entry.content)

		// Build expected Go function names
		for _, rustFn := range rustFns {
			expectedGoName := buildGoHTTPName(entry.modulePath, rustFn)
			if !goFnSet[expectedGoName] {
				allErrors = append(allErrors,
					entry.modulePath+": expected Go function "+expectedGoName+" (from Rust "+rustFn+")")
			}
		}
	}

	if len(allErrors) > 0 {
		t.Errorf("HTTP coverage errors (%d):\n  %s", len(allErrors), strings.Join(allErrors, "\n  "))
	}

	t.Logf("Checked %d Rust http.rs files", len(rustHTTPFiles))
}

type httpFileEntry struct {
	modulePath string // e.g., "agent_completions"
	content    string
}

func findHTTPFiles(t *testing.T, root string) []httpFileEntry {
	t.Helper()
	var result []httpFileEntry
	filepath.Walk(root, func(path string, info os.FileInfo, err error) error {
		if err != nil || info.IsDir() {
			return nil
		}
		if info.Name() == "http.rs" {
			rel, _ := filepath.Rel(root, filepath.Dir(path))
			modulePath := strings.ReplaceAll(filepath.ToSlash(rel), "/", "_")
			data, err := os.ReadFile(path)
			if err != nil {
				t.Fatalf("reading %s: %v", path, err)
			}
			result = append(result, httpFileEntry{
				modulePath: modulePath,
				content:    string(data),
			})
		}
		return nil
	})
	sort.Slice(result, func(i, j int) bool { return result[i].modulePath < result[j].modulePath })
	return result
}

func extractRustHTTPFunctions(content string) []string {
	re := regexp.MustCompile(`pub\s+async\s+fn\s+(\w+)`)
	matches := re.FindAllStringSubmatch(content, -1)
	var names []string
	for _, m := range matches {
		names = append(names, m[1])
	}
	return names
}

// buildGoHTTPName builds a PascalCase Go function name from module path + Rust fn name.
// e.g., modulePath="agent_completions", rustFn="create_agent_completion_unary" → "AgentCompletionsCreateAgentCompletionUnary"
func buildGoHTTPName(modulePath, rustFnName string) string {
	parts := strings.Split(modulePath, "_")
	fnParts := strings.Split(rustFnName, "_")
	all := append(parts, fnParts...)
	var b strings.Builder
	for _, p := range all {
		if p == "" {
			continue
		}
		runes := []rune(p)
		runes[0] = unicode.ToUpper(runes[0])
		b.WriteString(string(runes))
	}
	return b.String()
}

