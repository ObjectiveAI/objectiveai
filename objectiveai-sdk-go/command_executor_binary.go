package objectiveai

import (
	"bufio"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
)

// BinaryCommandExecutorOptions configures a BinaryCommandExecutor.
type BinaryCommandExecutorOptions struct {
	// ObjectiveaiDir is the layout root; the binary is
	// `<ObjectiveaiDir>/bin/objectiveai[.exe]`. Defaults to `~/.objectiveai`.
	ObjectiveaiDir string
	// ExtraEnv is layered onto the child's environment.
	ExtraEnv map[string]string
	// KillOnDrop kills the child when the stream is closed early.
	KillOnDrop bool
	// Detach detaches the child so it outlives the parent.
	Detach bool
}

// BinaryCommandExecutor spawns the objectiveai CLI as a child process and
// streams its stdout (line-delimited JSON, yielded raw — the typed CliStream
// discriminates errors vs responses). Port of binary.ts / binary.py.
//
// The request is serialized to JSON and passed via the cli's top-level
// `--request` flag; stdin is null, stdout is piped, stderr is inherited.
type BinaryCommandExecutor struct {
	options BinaryCommandExecutorOptions
}

// NewBinaryCommandExecutor constructs a BinaryCommandExecutor.
func NewBinaryCommandExecutor(options BinaryCommandExecutorOptions) *BinaryCommandExecutor {
	return &BinaryCommandExecutor{options: options}
}

// Execute spawns `<dir>/bin/objectiveai[.exe] --request <json>` and returns a
// RawStream over its stdout lines.
func (e *BinaryCommandExecutor) Execute(ctx context.Context, request any) (RawStream, error) {
	payload, err := json.Marshal(request)
	if err != nil {
		return nil, fmt.Errorf("objectiveai: marshal request: %w", err)
	}
	binary, err := e.resolveBinary()
	if err != nil {
		return nil, err
	}

	cmd := exec.CommandContext(ctx, binary, "--request", string(payload))
	cmd.Stdin = nil
	cmd.Stderr = os.Stderr // inherit the parent's stderr
	cmd.Env = os.Environ()
	for k, v := range e.options.ExtraEnv {
		cmd.Env = append(cmd.Env, k+"="+v)
	}
	if e.options.Detach {
		applyDetach(cmd)
	}

	stdout, err := cmd.StdoutPipe()
	if err != nil {
		return nil, fmt.Errorf("objectiveai: stdout pipe: %w", err)
	}
	if err := cmd.Start(); err != nil {
		return nil, fmt.Errorf("objectiveai: start cli: %w", err)
	}

	scanner := bufio.NewScanner(stdout)
	scanner.Buffer(make([]byte, 0, 1024*1024), 1024*1024) // 1MB lines, like Stream[T]
	return &binaryRawStream{
		cmd:        cmd,
		scanner:    scanner,
		killOnDrop: e.options.KillOnDrop,
		detach:     e.options.Detach,
	}, nil
}

func (e *BinaryCommandExecutor) resolveBinary() (string, error) {
	exe := "objectiveai"
	if runtime.GOOS == "windows" {
		exe = "objectiveai.exe"
	}
	dir := e.options.ObjectiveaiDir
	if dir == "" {
		home, err := os.UserHomeDir()
		if err != nil {
			return "", fmt.Errorf("objectiveai: resolve home dir: %w", err)
		}
		dir = filepath.Join(home, ".objectiveai")
	}
	return filepath.Join(dir, "bin", exe), nil
}

type binaryRawStream struct {
	cmd        *exec.Cmd
	scanner    *bufio.Scanner
	killOnDrop bool
	detach     bool
	done       bool
}

func (s *binaryRawStream) Next() (json.RawMessage, error) {
	for s.scanner.Scan() {
		line := s.scanner.Bytes()
		if len(trimSpace(line)) == 0 {
			continue
		}
		// Copy: scanner reuses its buffer across Scan calls.
		out := make(json.RawMessage, len(line))
		copy(out, line)
		return out, nil
	}
	if err := s.scanner.Err(); err != nil {
		return nil, err
	}
	return nil, io.EOF
}

func (s *binaryRawStream) Close() error {
	if s.done {
		return nil
	}
	s.done = true
	if s.killOnDrop && s.cmd.Process != nil {
		_ = s.cmd.Process.Kill()
	}
	if !s.detach {
		_ = s.cmd.Wait()
	}
	return nil
}

// trimSpace reports the line with leading/trailing ASCII whitespace removed,
// matching the JS/Py `.trim()`/`.strip()` blank-line skip without allocating.
func trimSpace(b []byte) []byte {
	start := 0
	for start < len(b) && isASCIISpace(b[start]) {
		start++
	}
	end := len(b)
	for end > start && isASCIISpace(b[end-1]) {
		end--
	}
	return b[start:end]
}

func isASCIISpace(c byte) bool {
	return c == ' ' || c == '\t' || c == '\n' || c == '\r' || c == '\v' || c == '\f'
}
