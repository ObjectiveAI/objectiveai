// Package objectiveai provides auto-generated Go types for the ObjectiveAI API.
//
// Types are generated from JSON Schema files in objectiveai-json-schema/
// by scripts/install_go.go. Do not edit generated files directly.
package objectiveai

import "github.com/go-playground/validator/v10"

// variantValidator is used by UnmarshalJSON on variant structs to validate
// that a deserialized value matches the variant's constraints (e.g., oneof).
//
// NOTE: The "dive" validate tag does not work with OrderedMap fields because
// go-playground/validator only dives into native maps/slices. OrderedMap values
// are validated server-side instead. RegisterCustomTypeFunc cannot be used
// because each generic instantiation is a different reflect.Type.
var variantValidator = validator.New()

// Described is implemented by every generated type.
// It provides metadata that cannot be derived from Go's type system.
// Methods use the Schema prefix to avoid conflicts with struct fields
// named "title" or "description".
type Described interface {
	SchemaTitle() string
	SchemaDescription() string
}

// SchemaBody provides the non-title/non-description parts of a schema
// for types where the structure cannot be inferred from Go reflection
// (anyOf interfaces, enums, arrays, primitives).
//
// Struct types do NOT implement this — their properties are derived
// from reflection on struct fields + tags by the roundtrip test.
type SchemaBody interface {
	Body() map[string]any
}
