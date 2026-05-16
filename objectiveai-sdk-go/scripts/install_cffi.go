// install_cffi.go
//
// Validates objectiveai-sdk-rs-cffi dist/ and copies the WASM binary
// into objectiveai-sdk-go/lib/ for Go embed.
//
// Delegates the fingerprint check to objectiveai-sdk-rs-cffi/validate.sh.
// If dist/ is missing or stale, exits with an error — run build.sh first.
//
//go:build ignore

package main

import (
	"fmt"
	"io"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
)

func main() {
	_, thisFile, _, _ := runtime.Caller(0)
	scriptsDir := filepath.Dir(thisFile)
	goRoot := filepath.Dir(scriptsDir)
	repoRoot := filepath.Dir(goRoot)

	cffiDir := filepath.Join(repoRoot, "objectiveai-sdk-rs-cffi")
	wasmSrc := filepath.Join(cffiDir, "dist", "objectiveai_cffi.wasm")
	validateScript := filepath.Join(cffiDir, "validate.sh")
	libDir := filepath.Join(goRoot, "lib")
	wasmDst := filepath.Join(libDir, "objectiveai_cffi.wasm")

	// 1. Validate dist/ is up to date
	cmd := exec.Command("bash", validateScript)
	cmd.Stdout = os.Stdout
	cmd.Stderr = os.Stderr
	if err := cmd.Run(); err != nil {
		fmt.Fprintf(os.Stderr, "objectiveai-sdk-rs-cffi dist/ is not valid. Run build.sh first.\n")
		os.Exit(1)
	}

	// 2. Copy WASM to lib/
	if err := os.MkdirAll(libDir, 0o755); err != nil {
		fmt.Fprintf(os.Stderr, "Failed to create lib dir: %v\n", err)
		os.Exit(1)
	}

	src, err := os.Open(wasmSrc)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Failed to open WASM: %v\n", err)
		os.Exit(1)
	}
	defer src.Close()

	dst, err := os.Create(wasmDst)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Failed to create WASM dest: %v\n", err)
		os.Exit(1)
	}
	defer dst.Close()

	n, err := io.Copy(dst, src)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Failed to copy WASM: %v\n", err)
		os.Exit(1)
	}

	fmt.Printf("Installed objectiveai_cffi.wasm (%d bytes)\n", n)
}
