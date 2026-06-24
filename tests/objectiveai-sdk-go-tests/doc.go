// Package tests is the Go SDK's HTTP/snapshot integration suite, split out
// of objectiveai-sdk-go into a standalone module that imports the built SDK
// (via the replace directive in go.mod) and drives a running API server.
//
// The server's base URL is supplied by the harness via OBJECTIVEAI_ADDRESS;
// these tests skip when it is unset. Snapshots are read from the shared
// corpus owned by the sibling objectiveai-api-tests project — they are never
// duplicated here. The non-network SDK unit tests (merge/push, schema
// roundtrip, cffi/http coverage) stay in objectiveai-sdk-go.
//
// This file exists only so the module's package always has a non-test source
// file.
package tests
