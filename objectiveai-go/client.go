package objectiveai

import (
	"bufio"
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"os"
	"strings"
)

const defaultAddress = "https://api.objectiveai.dev"

// Client is the HTTP client for the ObjectiveAI API.
type Client struct {
	Address                  string
	Authorization            string
	UserAgent                string
	HTTPReferer              string
	XTitle                   string
	XGithubAuthorization     string
	XOpenrouterAuthorization string
	XMCPAuthorization        map[string]string
	XViewerSignature         string
	XViewerAddress           string
	XCommitAuthorName        string
	XCommitAuthorEmail       string
	HTTPClient               *http.Client
}

// NewClient creates a new ObjectiveAI client.
// Fields fall back to environment variables, then defaults.
func NewClient(opts ...func(*Client)) *Client {
	c := &Client{
		Address:                  envOr("OBJECTIVEAI_ADDRESS", defaultAddress),
		Authorization:            os.Getenv("OBJECTIVEAI_AUTHORIZATION"),
		UserAgent:                os.Getenv("USER_AGENT"),
		HTTPReferer:              os.Getenv("HTTP_REFERER"),
		XTitle:                   os.Getenv("X_TITLE"),
		XGithubAuthorization:     os.Getenv("GITHUB_AUTHORIZATION"),
		XOpenrouterAuthorization: os.Getenv("OPENROUTER_AUTHORIZATION"),
		XViewerSignature:         os.Getenv("VIEWER_SIGNATURE"),
		XViewerAddress:           os.Getenv("VIEWER_ADDRESS"),
		XCommitAuthorName:        os.Getenv("COMMIT_AUTHOR_NAME"),
		XCommitAuthorEmail:       os.Getenv("COMMIT_AUTHOR_EMAIL"),
	}

	if mcp := os.Getenv("MCP_AUTHORIZATION"); mcp != "" {
		var m map[string]string
		if json.Unmarshal([]byte(mcp), &m) == nil {
			c.XMCPAuthorization = m
		}
	}

	for _, opt := range opts {
		opt(c)
	}

	if c.HTTPClient == nil {
		c.HTTPClient = &http.Client{}
	}

	return c
}

func envOr(key, fallback string) string {
	if v := os.Getenv(key); v != "" {
		return strings.TrimSpace(v)
	}
	return fallback
}

func (c *Client) buildURL(path string) string {
	base := strings.TrimRight(c.Address, "/")
	if !strings.HasPrefix(path, "/") {
		path = "/" + path
	}
	return base + path
}

func (c *Client) buildHeaders() http.Header {
	h := http.Header{}
	h.Set("Content-Type", "application/json")

	if c.Authorization != "" {
		auth := c.Authorization
		if !strings.HasPrefix(auth, "Bearer ") {
			auth = "Bearer " + auth
		}
		h.Set("Authorization", auth)
	}
	if c.UserAgent != "" {
		h.Set("User-Agent", c.UserAgent)
	}
	if c.HTTPReferer != "" {
		h.Set("HTTP-Referer", c.HTTPReferer)
	}
	if c.XTitle != "" {
		h.Set("X-Title", c.XTitle)
	}
	if c.XGithubAuthorization != "" {
		h.Set("X-GITHUB-AUTHORIZATION", c.XGithubAuthorization)
	}
	if c.XOpenrouterAuthorization != "" {
		h.Set("X-OPENROUTER-AUTHORIZATION", c.XOpenrouterAuthorization)
	}
	if len(c.XMCPAuthorization) > 0 {
		data, _ := json.Marshal(c.XMCPAuthorization)
		h.Set("X-MCP-AUTHORIZATION", string(data))
	}
	if c.XViewerSignature != "" {
		h.Set("X-VIEWER-SIGNATURE", c.XViewerSignature)
	}
	if c.XViewerAddress != "" {
		h.Set("X-VIEWER-ADDRESS", c.XViewerAddress)
	}
	if c.XCommitAuthorName != "" {
		h.Set("X-COMMIT-AUTHOR-NAME", c.XCommitAuthorName)
	}
	if c.XCommitAuthorEmail != "" {
		h.Set("X-COMMIT-AUTHOR-EMAIL", c.XCommitAuthorEmail)
	}

	return h
}

// isResponseError checks if a JSON object looks like a ResponseError.
func isResponseError(data []byte) bool {
	var obj struct {
		Code    *json.Number `json:"code"`
		Message any          `json:"message"`
	}
	if json.Unmarshal(data, &obj) != nil || obj.Code == nil {
		return false
	}
	return true
}

func newResponseError(statusCode int, rawBody []byte) *ErrorResponseError {
	if len(rawBody) > 0 && isResponseError(rawBody) {
		var resp ErrorResponseError
		if json.Unmarshal(rawBody, &resp) == nil {
			return &resp
		}
	}
	return &ErrorResponseError{
		Code:    uint32(statusCode),
		Message: JsonValue{Value: string(rawBody)},
	}
}

func (c *Client) doRequest(ctx context.Context, method, path string, body any, extraHeaders http.Header) (*http.Response, error) {
	var reqBody io.Reader
	if body != nil {
		data, err := json.Marshal(body)
		if err != nil {
			return nil, fmt.Errorf("objectiveai: marshal body: %w", err)
		}
		reqBody = bytes.NewReader(data)
	}

	req, err := http.NewRequestWithContext(ctx, method, c.buildURL(path), reqBody)
	if err != nil {
		return nil, fmt.Errorf("objectiveai: build request: %w", err)
	}
	req.Header = c.buildHeaders()
	for k, vs := range extraHeaders {
		for _, v := range vs {
			req.Header.Set(k, v)
		}
	}

	return c.HTTPClient.Do(req)
}

// PostUnary sends a POST request and deserializes the JSON response into T.
func PostUnary[T any](ctx context.Context, c *Client, path string, body any) (*T, error) {
	resp, err := c.doRequest(ctx, "POST", path, body, nil)
	if err != nil {
		return nil, err
	}
	defer resp.Body.Close()

	respBody, err := io.ReadAll(resp.Body)
	if err != nil {
		return nil, fmt.Errorf("objectiveai: read response: %w", err)
	}

	if resp.StatusCode < 200 || resp.StatusCode >= 300 {
		return nil, newResponseError(resp.StatusCode, respBody)
	}

	var result T
	if err := json.Unmarshal(respBody, &result); err != nil {
		return nil, fmt.Errorf("objectiveai: decode response: %w", err)
	}
	return &result, nil
}

// GetUnary sends a GET request and deserializes the JSON response into T.
func GetUnary[T any](ctx context.Context, c *Client, path string, body any) (*T, error) {
	resp, err := c.doRequest(ctx, "GET", path, body, nil)
	if err != nil {
		return nil, err
	}
	defer resp.Body.Close()

	respBody, err := io.ReadAll(resp.Body)
	if err != nil {
		return nil, fmt.Errorf("objectiveai: read response: %w", err)
	}

	if resp.StatusCode < 200 || resp.StatusCode >= 300 {
		return nil, newResponseError(resp.StatusCode, respBody)
	}

	var result T
	if err := json.Unmarshal(respBody, &result); err != nil {
		return nil, fmt.Errorf("objectiveai: decode response: %w", err)
	}
	return &result, nil
}

// DeleteUnary sends a DELETE request and deserializes the JSON response into T.
func DeleteUnary[T any](ctx context.Context, c *Client, path string, body any) (*T, error) {
	resp, err := c.doRequest(ctx, "DELETE", path, body, nil)
	if err != nil {
		return nil, err
	}
	defer resp.Body.Close()

	respBody, err := io.ReadAll(resp.Body)
	if err != nil {
		return nil, fmt.Errorf("objectiveai: read response: %w", err)
	}

	if resp.StatusCode < 200 || resp.StatusCode >= 300 {
		return nil, newResponseError(resp.StatusCode, respBody)
	}

	var result T
	if err := json.Unmarshal(respBody, &result); err != nil {
		return nil, fmt.Errorf("objectiveai: decode response: %w", err)
	}
	return &result, nil
}

// DeleteNoContent sends a DELETE request that expects no response body.
func DeleteNoContent(ctx context.Context, c *Client, path string, body any) error {
	resp, err := c.doRequest(ctx, "DELETE", path, body, nil)
	if err != nil {
		return err
	}
	defer resp.Body.Close()

	if resp.StatusCode < 200 || resp.StatusCode >= 300 {
		respBody, _ := io.ReadAll(resp.Body)
		return newResponseError(resp.StatusCode, respBody)
	}
	return nil
}

// Stream is a typed SSE event stream from the API.
// Call Next() to read events, Close() when done.
// The context passed to PostStreaming controls cancellation.
type Stream[T any] struct {
	resp    *http.Response
	scanner *bufio.Scanner
	done    bool
}

// PostStreaming sends a POST request and returns a typed SSE event stream.
func PostStreaming[T any](ctx context.Context, c *Client, path string, body any) (*Stream[T], error) {
	extra := http.Header{}
	extra.Set("Accept", "text/event-stream")

	resp, err := c.doRequest(ctx, "POST", path, body, extra)
	if err != nil {
		return nil, err
	}

	if resp.StatusCode < 200 || resp.StatusCode >= 300 {
		defer resp.Body.Close()
		respBody, _ := io.ReadAll(resp.Body)
		return nil, newResponseError(resp.StatusCode, respBody)
	}

	scanner := bufio.NewScanner(resp.Body)
	scanner.Buffer(make([]byte, 0, 1024*1024), 1024*1024) // 1MB buffer

	return &Stream[T]{resp: resp, scanner: scanner}, nil
}

// Next reads the next event from the stream.
// Returns the parsed event, or an error. Returns io.EOF when complete.
func (s *Stream[T]) Next() (*T, error) {
	for !s.done {
		event, err := s.nextRawEvent()
		if err != nil {
			return nil, err
		}
		if event == nil {
			continue
		}

		// Check for API error in stream
		if isResponseError(event) {
			var resp ErrorResponseError
			if json.Unmarshal(event, &resp) == nil {
				return nil, &resp
			}
		}

		var result T
		if err := json.Unmarshal(event, &result); err != nil {
			return nil, fmt.Errorf("objectiveai: decode stream event: %w", err)
		}
		return &result, nil
	}
	return nil, io.EOF
}

// nextRawEvent reads the next raw SSE data payload.
func (s *Stream[T]) nextRawEvent() (json.RawMessage, error) {
	var dataLines []string

	for s.scanner.Scan() {
		line := s.scanner.Text()

		if line == "" {
			// Empty line = event boundary
			if len(dataLines) == 0 {
				continue
			}
			data := strings.Join(dataLines, "\n")
			dataLines = nil

			if data == "[DONE]" {
				s.done = true
				return nil, io.EOF
			}
			if data == "" {
				continue
			}
			return json.RawMessage(data), nil
		}

		// Skip comment lines
		if strings.HasPrefix(line, ":") {
			continue
		}

		// Parse data: lines
		if strings.HasPrefix(line, "data:") {
			data := strings.TrimPrefix(line[5:], " ")
			dataLines = append(dataLines, data)
			continue
		}

		// Ignore other SSE fields (event:, id:, retry:)
	}

	if err := s.scanner.Err(); err != nil {
		return nil, err
	}

	// Handle remaining data
	if len(dataLines) > 0 {
		data := strings.Join(dataLines, "\n")
		if data != "[DONE]" && data != "" {
			return json.RawMessage(data), nil
		}
	}

	s.done = true
	return nil, io.EOF
}

// Close closes the underlying response body.
func (s *Stream[T]) Close() error {
	s.done = true
	return s.resp.Body.Close()
}
