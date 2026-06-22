package objectiveai

import (
	"bufio"
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"os"
	"strconv"
	"sync"
	"sync/atomic"
)

// PluginCommandExecutor lets a plugin (run by `plugins run`) ask the HOST to run
// CLI commands over the process's own stdin/stdout, speaking the NDJSON command
// protocol. Direct port of plugin.ts / plugin.py (themselves ports of the Rust
// PluginCommandExecutor).
//
// It reproduces the four primitives that make PARALLEL callers safe (many
// concurrent Execute calls share ONE stdin/stdout without corrupting it):
//
//  1. ONE reader. A single goroutine owns os.Stdin; no caller reads stdin
//     directly, so concurrent calls never race on the read.
//  2. id -> channel demux. Each call mints a monotonic id and registers an
//     (unbounded) channel; the reader routes each line to the matching id.
//  3. Serialized writes. All writes to stdout go through one mutex so two
//     concurrent callers can't interleave partial NDJSON lines.
//  4. Liveness recheck. On stdin EOF the reader sets alive=false and ends every
//     pending channel; each Execute inserts THEN rechecks alive, so a call
//     racing the close can't strand a registered channel.
//
// As in Rust/JS/Py there is exactly one instance per process (it captures the
// global stdin/stdout): use PluginExecutorInstance.
type PluginCommandExecutor struct {
	counter atomic.Uint64
	mu      sync.Mutex // guards pending
	pending map[string]*pluginChannel
	alive   atomic.Bool
	writeMu sync.Mutex
	reader  sync.Once
	in      io.Reader
	out     io.Writer
}

var (
	pluginInstance     *PluginCommandExecutor
	pluginInstanceOnce sync.Once
)

// PluginExecutorInstance returns the process-wide singleton (it captures the
// global stdin/stdout).
func PluginExecutorInstance() *PluginCommandExecutor {
	pluginInstanceOnce.Do(func() {
		pluginInstance = &PluginCommandExecutor{
			pending: make(map[string]*pluginChannel),
			in:      os.Stdin,
			out:     os.Stdout,
		}
		pluginInstance.alive.Store(true)
	})
	return pluginInstance
}

// Execute sends `request` to the host and streams back its responses (yielded
// raw; the typed CliStream discriminates errors vs responses). Safe to call
// concurrently from any number of parallel callers.
func (p *PluginCommandExecutor) Execute(ctx context.Context, request any) (RawStream, error) {
	p.ensureReader()

	id := strconv.FormatUint(p.counter.Add(1)-1, 10)
	ch := newPluginChannel()
	// (2) register before doing anything else, then (4) recheck liveness.
	p.mu.Lock()
	p.pending[id] = ch
	p.mu.Unlock()
	if !p.alive.Load() {
		p.removePending(id)
		return nil, errors.New("plugin executor: stdin closed")
	}

	payload, err := json.Marshal(request)
	if err != nil {
		p.removePending(id)
		return nil, fmt.Errorf("objectiveai: marshal request: %w", err)
	}
	// Serialize the request as the cli's `--request` argv; the host re-enters
	// its `run` with this command. Same wire shape as JS/Py.
	line, err := json.Marshal(struct {
		Type    string   `json:"type"`
		ID      string   `json:"id"`
		Command []string `json:"command"`
	}{Type: "command", ID: id, Command: []string{"--request", string(payload)}})
	if err != nil {
		p.removePending(id)
		return nil, fmt.Errorf("objectiveai: marshal command: %w", err)
	}
	if err := p.write(line); err != nil { // (3) serialized write
		p.removePending(id)
		return nil, err
	}

	return &pluginRawStream{exec: p, id: id, ch: ch, ctx: ctx}, nil
}

func (p *PluginCommandExecutor) write(line []byte) error {
	p.writeMu.Lock()
	defer p.writeMu.Unlock()
	if _, err := p.out.Write(append(line, '\n')); err != nil {
		return err
	}
	return nil
}

func (p *PluginCommandExecutor) ensureReader() {
	p.reader.Do(func() { go p.readLoop() })
}

func (p *PluginCommandExecutor) readLoop() {
	scanner := bufio.NewScanner(p.in)
	scanner.Buffer(make([]byte, 0, 1024*1024), 1024*1024)
	for scanner.Scan() {
		p.onLine(scanner.Bytes())
	}
	p.onEOF()
}

// onLine routes one inbound NDJSON line to the matching id's channel.
func (p *PluginCommandExecutor) onLine(raw []byte) {
	line := trimSpace(raw)
	if len(line) == 0 {
		return
	}
	var obj map[string]json.RawMessage
	if json.Unmarshal(line, &obj) != nil {
		return
	}
	idRaw, ok := obj["id"]
	if !ok {
		return
	}
	var id string
	if json.Unmarshal(idRaw, &id) != nil {
		return
	}
	p.mu.Lock()
	ch := p.pending[id]
	p.mu.Unlock()
	if ch == nil {
		return
	}

	value, hasValue := obj["value"]
	// Terminal markers: SDK form {id, done:true} or host form
	// {id, value:{type:"command_complete", exit_code}}.
	done := false
	if doneRaw, ok := obj["done"]; ok {
		_ = json.Unmarshal(doneRaw, &done)
	}
	if done || (hasValue && isCommandComplete(value)) {
		p.removePending(id)
		ch.end()
		return
	}
	// Yield each value item RAW; error envelopes pass through as values and the
	// caller's CliStream discriminates them via the CliError union.
	if hasValue {
		cp := make(json.RawMessage, len(value))
		copy(cp, value)
		ch.push(cp)
	}
}

func (p *PluginCommandExecutor) onEOF() {
	// (4) flag before draining, mirroring the Rust/JS/Py ordering.
	p.alive.Store(false)
	p.mu.Lock()
	pending := p.pending
	p.pending = make(map[string]*pluginChannel)
	p.mu.Unlock()
	for _, ch := range pending {
		ch.end()
	}
}

func (p *PluginCommandExecutor) removePending(id string) {
	p.mu.Lock()
	delete(p.pending, id)
	p.mu.Unlock()
}

func isCommandComplete(raw json.RawMessage) bool {
	if len(raw) == 0 {
		return false
	}
	var probe struct {
		Type string `json:"type"`
	}
	return json.Unmarshal(raw, &probe) == nil && probe.Type == "command_complete"
}

// pluginChannel is a single-producer / single-consumer UNBOUNDED queue (the Go
// analogue of the JS makeChannel / Python asyncio.Queue). Unbounded so a slow
// consumer can't block the shared reader goroutine and stall every other id.
type pluginChannel struct {
	mu     sync.Mutex
	queue  []json.RawMessage
	done   bool
	err    error
	notify chan struct{}
}

func newPluginChannel() *pluginChannel {
	return &pluginChannel{notify: make(chan struct{}, 1)}
}

func (c *pluginChannel) signal() {
	select {
	case c.notify <- struct{}{}:
	default:
	}
}

func (c *pluginChannel) push(v json.RawMessage) {
	c.mu.Lock()
	if c.done {
		c.mu.Unlock()
		return
	}
	c.queue = append(c.queue, v)
	c.mu.Unlock()
	c.signal()
}

func (c *pluginChannel) end() {
	c.mu.Lock()
	c.done = true
	c.mu.Unlock()
	c.signal()
}

func (c *pluginChannel) recv(ctx context.Context) (json.RawMessage, error) {
	for {
		c.mu.Lock()
		if len(c.queue) > 0 {
			v := c.queue[0]
			c.queue = c.queue[1:]
			c.mu.Unlock()
			return v, nil
		}
		if c.err != nil {
			err := c.err
			c.mu.Unlock()
			return nil, err
		}
		if c.done {
			c.mu.Unlock()
			return nil, io.EOF
		}
		c.mu.Unlock()
		select {
		case <-c.notify:
		case <-ctx.Done():
			return nil, ctx.Err()
		}
	}
}

type pluginRawStream struct {
	exec *PluginCommandExecutor
	id   string
	ch   *pluginChannel
	ctx  context.Context
	done bool
}

func (s *pluginRawStream) Next() (json.RawMessage, error) {
	return s.ch.recv(s.ctx)
}

func (s *pluginRawStream) Close() error {
	if s.done {
		return nil
	}
	s.done = true
	s.exec.removePending(s.id)
	s.ch.end()
	return nil
}
