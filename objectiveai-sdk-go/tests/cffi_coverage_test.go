package tests

import (
	"os"
	"path/filepath"
	"regexp"
	"strings"
	"testing"
	"unicode"
)

// TestCFFICoverage asserts that cffi.go has an exported Go function for every
// extern "C" function in objectiveai-rs-cffi/src/lib.rs.
//
// Rust function names like objectiveai_validate_ensemble are expected to appear
// in cffi.go as PascalCase Go functions like ValidateEnsemble.
func TestCFFICoverage(t *testing.T) {
	// Locate repo root (tests/ -> objectiveai-go/ -> repo root)
	repoRoot := RepoRoot()

	// Read Rust lib.rs
	rustPath := filepath.Join(repoRoot, "objectiveai-rs-cffi", "src", "lib.rs")
	rustBytes, err := os.ReadFile(rustPath)
	if err != nil {
		t.Fatalf("Failed to read %s: %v", rustPath, err)
	}

	// Extract all extern "C" function names (objectiveai_*)
	rustFnRe := regexp.MustCompile(`pub\s+unsafe\s+extern\s+"C"\s+fn\s+(objectiveai_\w+)`)
	rustMatches := rustFnRe.FindAllStringSubmatch(string(rustBytes), -1)
	if len(rustMatches) == 0 {
		t.Fatal("Found no extern \"C\" functions in lib.rs")
	}

	var rustFns []string
	for _, m := range rustMatches {
		rustFns = append(rustFns, m[1])
	}

	// Read Go cffi.go
	goPath := filepath.Join(repoRoot, "objectiveai-go", "cffi.go")
	goBytes, err := os.ReadFile(goPath)
	if err != nil {
		t.Fatalf("Failed to read %s: %v", goPath, err)
	}
	goSrc := string(goBytes)

	// Extract all exported Go function names from cffi.go
	goFnRe := regexp.MustCompile(`func\s+([A-Z]\w*)\s*\(`)
	goMatches := goFnRe.FindAllStringSubmatch(goSrc, -1)
	goFnSet := make(map[string]bool)
	for _, m := range goMatches {
		goFnSet[m[1]] = true
	}

	// Build the set of expected Go function names from Rust.
	expectedGoFns := make(map[string]bool)
	for _, rustFn := range rustFns {
		expectedGoFns[snakeToPascal(strings.TrimPrefix(rustFn, "objectiveai_"))] = true
	}

	// For each Rust function, assert the PascalCase equivalent exists in Go.
	var missing []string
	for _, rustFn := range rustFns {
		goName := snakeToPascal(strings.TrimPrefix(rustFn, "objectiveai_"))
		if !goFnSet[goName] {
			missing = append(missing, rustFn+" -> "+goName)
		}
	}

	if len(missing) > 0 {
		t.Errorf("cffi.go is missing %d function(s):\n  %s",
			len(missing), strings.Join(missing, "\n  "))
	}

	// Assert no extra exported functions exist in cffi.go beyond what Rust declares.
	var extra []string
	for goFn := range goFnSet {
		if !expectedGoFns[goFn] {
			extra = append(extra, goFn)
		}
	}

	if len(extra) > 0 {
		t.Errorf("cffi.go has %d unexpected exported function(s):\n  %s",
			len(extra), strings.Join(extra, "\n  "))
	}

	t.Logf("Checked %d Rust functions, %d Go exported functions found", len(rustFns), len(goFnSet))
}

// snakeToPascal converts a snake_case string to PascalCase.
// e.g. "validate_ensemble" -> "ValidateEnsemble"
func snakeToPascal(s string) string {
	var b strings.Builder
	upper := true
	for _, r := range s {
		if r == '_' {
			upper = true
			continue
		}
		if upper {
			b.WriteRune(unicode.ToUpper(r))
			upper = false
		} else {
			b.WriteRune(r)
		}
	}
	return b.String()
}
