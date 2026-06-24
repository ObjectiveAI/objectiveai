module github.com/ObjectiveAI/objectiveai/tests/objectiveai-sdk-go-tests

go 1.26.1

require (
	github.com/ObjectiveAI/objectiveai/objectiveai-sdk-go v0.0.0
	github.com/wk8/go-ordered-map/v2 v2.1.8
)

require (
	github.com/bahlo/generic-list-go v0.2.0 // indirect
	github.com/buger/jsonparser v1.1.1 // indirect
	github.com/gabriel-vasile/mimetype v1.4.12 // indirect
	github.com/go-playground/locales v0.14.1 // indirect
	github.com/go-playground/universal-translator v0.18.1 // indirect
	github.com/go-playground/validator/v10 v10.30.1 // indirect
	github.com/google/uuid v1.6.0 // indirect
	github.com/leodido/go-urn v1.4.0 // indirect
	github.com/mailru/easyjson v0.7.7 // indirect
	github.com/tetratelabs/wazero v1.11.0 // indirect
	golang.org/x/crypto v0.46.0 // indirect
	golang.org/x/sys v0.39.0 // indirect
	golang.org/x/text v0.32.0 // indirect
	gopkg.in/yaml.v3 v3.0.1 // indirect
)

// This is an importer of the built Go SDK: it depends on the published
// module path but resolves it to the in-repo source so the tests run
// against the freshly-built SDK.
replace github.com/ObjectiveAI/objectiveai/objectiveai-sdk-go => ../../objectiveai-sdk-go
