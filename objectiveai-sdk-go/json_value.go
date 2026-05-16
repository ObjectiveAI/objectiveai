package objectiveai

import "encoding/json"

// JsonValue wraps any JSON value. It serializes and deserializes transparently
// as the underlying value (object, array, string, number, boolean, or null).
// Unlike bare `any`, JsonValue is a concrete type that can have methods.
type JsonValue struct {
	Value any `json:"value"`
}

func (v JsonValue) MarshalJSON() ([]byte, error) {
	return json.Marshal(v.Value)
}

func (v *JsonValue) UnmarshalJSON(data []byte) error {
	return json.Unmarshal(data, &v.Value)
}
